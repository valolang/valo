//! Initial dataflow over resolved HIR. Immutable borrows are emitted for
//! source-level `ByRef ReadOnly`; moves are representable internally only.
use super::type_properties::KnownProperty;
use super::typed_hir::{
    DisposeMethodId, DoCondition, Expression, ExpressionKind, InitialState, LocalId, LocalStorage,
    Place, PlaceOverlap, ScopeCleanup, ScopeId, Statement, TypedBody,
};
use crate::frontend::type_model::TypeName;
use crate::runtime::{Diagnostic, DiagnosticCode, Span};

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Uninitialized,
    Initialized,
    Moved,
    MaybeUnavailable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BorrowKind {
    Immutable,
    Mutable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupAction {
    Drop,
    Dispose,
    NoDrop,
    SkipUninitialized,
    SkipMoved,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    Normal,
    Return,
    ExitLoop,
    ContinueLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitCleanup {
    pub kind: ExitKind,
    pub span: Span,
    pub exited_scopes: Vec<ScopeId>,
    pub locals: Vec<(LocalId, CleanupAction)>,
    pub disposals: Vec<(LocalId, DisposeMethodId, CleanupAction)>,
    /// Ordered actions from the typed transfer plan, including Finally.
    pub handlers: Vec<HandlerDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerDecision {
    Dispose(LocalId, DisposeMethodId, CleanupAction),
    Finally(ScopeId),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnershipReport {
    pub exits: Vec<ExitCleanup>,
    pub replacements: Vec<ReplacementCleanup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementCleanup {
    pub span: Span,
    pub place: Place,
    pub old_value: CleanupAction,
}

pub fn check_body(body: &TypedBody) -> Result<(), Diagnostic> {
    analyze_body(body).map(|_| ())
}

pub fn analyze_body(body: &TypedBody) -> Result<OwnershipReport, Diagnostic> {
    let mut states = body
        .locals
        .iter()
        .map(|local| match local.initial_state {
            InitialState::Initialized => State::Initialized,
            InitialState::Uninitialized => State::Uninitialized,
        })
        .collect();
    let mut report = OwnershipReport::default();
    check_statements(body, &body.statements, &mut states, &mut report)?;
    Ok(report)
}

fn check_statements(
    body: &TypedBody,
    statements: &[Statement],
    states: &mut Vec<State>,
    report: &mut OwnershipReport,
) -> Result<bool, Diagnostic> {
    let mut terminated = false;
    for statement in statements {
        if terminated {
            break;
        }
        match statement {
            Statement::Initialize {
                target,
                value,
                span,
            } => {
                check_expression(body, value, states)?;
                if states[target.0] != State::Uninitialized {
                    return Err(error("Local is initialized more than once", *span));
                }
                states[target.0] = State::Initialized;
            }
            Statement::Store {
                target,
                value,
                span,
            } => {
                if let ExpressionKind::Move(source) = &value.kind
                    && let (Some(destination), Some(source)) =
                        (semantic_place(target), semantic_place(source))
                    && destination.overlap(source) != PlaceOverlap::Disjoint
                    && body.locals[source.root.0].properties.copy == KnownProperty::No
                {
                    return Err(error(
                        "Cannot assign a non-Copy place from Move of itself",
                        *span,
                    ));
                }
                check_expression(body, value, states)?;
                let place = semantic_place(target)
                    .ok_or_else(|| error("Store target is not a resolved place", *span))?;
                check_place_indices(body, place, states)?;
                let local = place.root;
                if body.locals[local.0].storage == LocalStorage::BorrowedImmutable {
                    return Err(error("Cannot assign through a ReadOnly reference", *span));
                }
                let old_value = if place.projections.is_empty() {
                    cleanup_action(body, local, states[local.0])
                } else {
                    CleanupAction::Unresolved
                };
                report.replacements.push(ReplacementCleanup {
                    span: *span,
                    place: place.clone(),
                    old_value,
                });
                if !place.projections.is_empty() {
                    require_initialized(states[local.0], &body.locals[local.0].name, *span)?;
                } else {
                    states[local.0] = State::Initialized;
                }
            }
            Statement::Return {
                value,
                exited_scopes,
                span,
                ..
            } => {
                check_expression(body, value, states)?;
                record_exit(body, ExitKind::Return, exited_scopes, *span, states, report);
                terminated = true;
            }
            Statement::ReturnVoid {
                exited_scopes,
                span,
                ..
            } => {
                record_exit(body, ExitKind::Return, exited_scopes, *span, states, report);
                terminated = true;
            }
            Statement::CallSub {
                function,
                signature,
                arguments,
                span,
            } => {
                check_expression(
                    body,
                    &Expression {
                        kind: ExpressionKind::Call {
                            function: *function,
                            signature: signature.clone(),
                            arguments: arguments.clone(),
                        },
                        ty: TypeName::Void,
                        category: super::typed_hir::ValueCategory::Value,
                        span: *span,
                    },
                    states,
                )?;
            }
            Statement::If {
                condition,
                then_scope,
                then_body,
                else_scope,
                else_body,
                span,
                ..
            } => {
                check_expression(body, condition, states)?;
                let mut then_states = states.clone();
                let mut else_states = states.clone();
                let then_ends = check_statements(body, then_body, &mut then_states, report)?;
                let else_ends = check_statements(body, else_body, &mut else_states, report)?;
                if !then_ends {
                    record_exit(
                        body,
                        ExitKind::Normal,
                        &[*then_scope],
                        *span,
                        &then_states,
                        report,
                    );
                }
                if !else_ends {
                    record_exit(
                        body,
                        ExitKind::Normal,
                        &[*else_scope],
                        *span,
                        &else_states,
                        report,
                    );
                }
                match (then_ends, else_ends) {
                    (true, true) => terminated = true,
                    (true, false) => *states = else_states,
                    (false, true) => *states = then_states,
                    (false, false) => merge(states, &then_states, &else_states),
                }
            }
            Statement::TryFinally {
                try_scope,
                try_body,
                finally_scope,
                finally_body,
                span,
            } => {
                let try_terminated = check_statements(body, try_body, states, report)?;
                if !try_terminated {
                    record_exit(body, ExitKind::Normal, &[*try_scope], *span, states, report);
                }
                if check_statements(body, finally_body, states, report)? {
                    return Err(error(
                        "Non-local exit inside typed Finally is not supported",
                        *span,
                    ));
                }
                record_exit(
                    body,
                    ExitKind::Normal,
                    &[*finally_scope],
                    *span,
                    states,
                    report,
                );
                terminated = try_terminated;
            }
            Statement::TryCatch {
                try_scope,
                try_body,
                catch_scope,
                catch_body,
                finally_scope,
                finally_body,
                span,
                ..
            } => {
                let mut try_states = states.clone();
                let mut catch_states = states.clone();
                let try_terminated = check_statements(body, try_body, &mut try_states, report)?;
                let catch_terminated =
                    check_statements(body, catch_body, &mut catch_states, report)?;
                if !try_terminated {
                    record_exit(
                        body,
                        ExitKind::Normal,
                        &[*try_scope],
                        *span,
                        &try_states,
                        report,
                    );
                }
                if !catch_terminated {
                    record_exit(
                        body,
                        ExitKind::Normal,
                        &[*catch_scope],
                        *span,
                        &catch_states,
                        report,
                    );
                }
                match (try_terminated, catch_terminated) {
                    (true, true) => terminated = true,
                    (true, false) => *states = catch_states,
                    (false, true) => *states = try_states,
                    (false, false) => merge(states, &try_states, &catch_states),
                }
                if let Some(finally_scope) = finally_scope {
                    if check_statements(body, finally_body, states, report)? {
                        return Err(error(
                            "Non-local exit inside typed Finally is not supported",
                            *span,
                        ));
                    }
                    record_exit(
                        body,
                        ExitKind::Normal,
                        &[*finally_scope],
                        *span,
                        states,
                        report,
                    );
                }
            }
            Statement::UsingDispose {
                body_scope,
                body: using_body,
                span,
                ..
            } => {
                let ended = check_statements(body, using_body, states, report)?;
                if !ended {
                    record_exit(
                        body,
                        ExitKind::Normal,
                        &[*body_scope],
                        *span,
                        states,
                        report,
                    );
                }
                terminated = ended;
            }
            Statement::While {
                condition,
                body_scope,
                body: loop_body,
                span,
                ..
            } => {
                check_expression(body, condition, states)?;
                check_loop_body(body, loop_body, *body_scope, *span, states, report)?;
            }
            Statement::Do {
                condition,
                body_scope,
                body: loop_body,
                span,
                ..
            } => {
                let expression = match condition {
                    DoCondition::Infinite => None,
                    DoCondition::PreWhile(e)
                    | DoCondition::PreUntil(e)
                    | DoCondition::PostWhile(e)
                    | DoCondition::PostUntil(e) => Some(e),
                };
                if let Some(expression) = expression {
                    check_expression(body, expression, states)?;
                }
                check_loop_body(body, loop_body, *body_scope, *span, states, report)?;
            }
            Statement::For {
                variable,
                start,
                end,
                step,
                body_scope,
                body: loop_body,
                span,
                ..
            } => {
                check_expression(body, start, states)?;
                check_expression(body, end, states)?;
                check_expression(body, step, states)?;
                states[variable.0] = State::Initialized;
                check_loop_body(body, loop_body, *body_scope, *span, states, report)?;
            }
            Statement::ForEach {
                variable,
                iterable,
                body_scope,
                body: loop_body,
                span,
                ..
            } => {
                check_expression(body, iterable, states)?;
                states[variable.0] = State::Initialized;
                check_loop_body(body, loop_body, *body_scope, *span, states, report)?;
            }
            Statement::ExitLoop {
                exited_scopes,
                span,
                ..
            } => {
                record_exit(
                    body,
                    ExitKind::ExitLoop,
                    exited_scopes,
                    *span,
                    states,
                    report,
                );
                terminated = true;
            }
            Statement::ContinueLoop {
                exited_scopes,
                span,
                ..
            } => {
                record_exit(
                    body,
                    ExitKind::ContinueLoop,
                    exited_scopes,
                    *span,
                    states,
                    report,
                );
                terminated = true;
            }
        }
    }
    Ok(terminated)
}

fn check_loop_body(
    body: &TypedBody,
    loop_body: &[Statement],
    body_scope: ScopeId,
    span: Span,
    states: &mut [State],
    report: &mut OwnershipReport,
) -> Result<(), Diagnostic> {
    let before = states.to_vec();
    let mut after = before.clone();
    let terminated = check_statements(body, loop_body, &mut after, report)?;
    if !terminated {
        record_exit(body, ExitKind::Normal, &[body_scope], span, &after, report);
    }
    // A loop can execute zero or multiple times. A move in the body cannot
    // establish a definite state for code following the loop.
    merge(states, &before, &after);
    Ok(())
}

fn record_exit(
    body: &TypedBody,
    kind: ExitKind,
    exited_scopes: &[ScopeId],
    span: Span,
    states: &[State],
    report: &mut OwnershipReport,
) {
    let locals = body
        .owned_exit_locals(exited_scopes)
        .into_iter()
        .map(|id| (id, cleanup_action(body, id, states[id.0])))
        .collect();
    let handlers: Vec<_> = body
        .cleanup_chain(exited_scopes)
        .into_iter()
        .map(|step| match step.action {
            ScopeCleanup::ExplicitDispose { resource, method } => {
                let action = match states[resource.0] {
                    State::Initialized => CleanupAction::Dispose,
                    State::Moved => CleanupAction::SkipMoved,
                    State::Uninitialized => CleanupAction::SkipUninitialized,
                    State::MaybeUnavailable => CleanupAction::Unresolved,
                };
                HandlerDecision::Dispose(resource, method, action)
            }
            ScopeCleanup::FinallyRegion { finally_scope } => {
                HandlerDecision::Finally(finally_scope)
            }
        })
        .collect();
    let disposals = handlers
        .iter()
        .filter_map(|handler| match handler {
            HandlerDecision::Dispose(resource, method, action) => {
                Some((*resource, *method, *action))
            }
            HandlerDecision::Finally(_) => None,
        })
        .collect();
    report.exits.push(ExitCleanup {
        kind,
        span,
        exited_scopes: exited_scopes.to_vec(),
        locals,
        disposals,
        handlers,
    });
}

fn cleanup_action(body: &TypedBody, id: LocalId, state: State) -> CleanupAction {
    match state {
        State::Uninitialized => CleanupAction::SkipUninitialized,
        State::Moved => CleanupAction::SkipMoved,
        State::MaybeUnavailable => CleanupAction::Unresolved,
        State::Initialized => match body.locals[id.0].properties.requires_drop {
            KnownProperty::Yes => CleanupAction::Drop,
            KnownProperty::No => CleanupAction::NoDrop,
            KnownProperty::Unknown => CleanupAction::Unresolved,
        },
    }
}

fn merge(destination: &mut [State], left: &[State], right: &[State]) {
    for (index, state) in destination.iter_mut().enumerate() {
        *state = if left[index] == right[index] {
            left[index]
        } else {
            State::MaybeUnavailable
        };
    }
}

fn check_expression(
    body: &TypedBody,
    expression: &Expression,
    states: &mut Vec<State>,
) -> Result<(), Diagnostic> {
    match &expression.kind {
        ExpressionKind::Constant(_) | ExpressionKind::ArrayInit { .. } => Ok(()),
        ExpressionKind::Tuple(values) => {
            for value in values {
                check_expression(body, value, states)?;
            }
            Ok(())
        }
        ExpressionKind::Place(place) => check_place_indices(body, place, states),
        ExpressionKind::Load(place)
        | ExpressionKind::BorrowImmutable(place)
        | ExpressionKind::BorrowMutable(place) => {
            let place = semantic_place(place).ok_or_else(|| {
                error(
                    "Reference does not designate a resolved place",
                    expression.span,
                )
            })?;
            check_place_indices(body, place, states)?;
            let local = place.root;
            require_initialized(states[local.0], &body.locals[local.0].name, expression.span)?;
            if matches!(expression.kind, ExpressionKind::BorrowMutable(_))
                && body.locals[local.0].storage == LocalStorage::BorrowedImmutable
            {
                return Err(error(
                    "Cannot mutably borrow a ReadOnly reference",
                    expression.span,
                ));
            }
            Ok(())
        }
        ExpressionKind::Move(place) => {
            let local = local_place(place)
                .ok_or_else(|| error("Move requires a local place", expression.span))?;
            require_initialized(states[local.0], &body.locals[local.0].name, expression.span)?;
            if body.locals[local.0].storage != LocalStorage::Value {
                return Err(error(
                    "Cannot move out of a borrowed reference",
                    expression.span,
                ));
            }
            match body.locals[local.0].properties.copy {
                KnownProperty::Yes => {} // Explicit Move of a Copy value is an ordinary copy.
                KnownProperty::No => states[local.0] = State::Moved,
                KnownProperty::Unknown => {
                    return Err(error(
                        "Move requires a resolved Copy/non-Copy classification",
                        expression.span,
                    ));
                }
            }
            Ok(())
        }
        ExpressionKind::Convert { value, .. } => check_expression(body, value, states),
        ExpressionKind::Arithmetic { left, right, .. }
        | ExpressionKind::Compare { left, right, .. } => {
            check_expression(body, left, states)?;
            check_expression(body, right, states)
        }
        ExpressionKind::Call { arguments, .. } => {
            let mut borrowed = Vec::<(Place, BorrowKind)>::new();
            for argument in arguments {
                let borrow = match &argument.value.kind {
                    ExpressionKind::BorrowMutable(place) => {
                        semantic_place(place).map(|place| (place.clone(), BorrowKind::Mutable))
                    }
                    ExpressionKind::BorrowImmutable(place) => {
                        semantic_place(place).map(|place| (place.clone(), BorrowKind::Immutable))
                    }
                    _ => None,
                };
                if let Some((place, kind)) = borrow {
                    if borrowed.iter().any(|(previous, previous_kind)| {
                        place.overlap(previous) != PlaceOverlap::Disjoint
                            && (*previous_kind == BorrowKind::Mutable
                                || kind == BorrowKind::Mutable)
                    }) {
                        return Err(error("The same local cannot be passed to conflicting ByRef parameters in one call", argument.value.span)
                            .with_help("Pass distinct variables so mutable ByRef access is exclusive"));
                    }
                    borrowed.push((place, kind));
                }
                check_expression(body, &argument.value, states)?;
            }
            Ok(())
        }
    }
}

fn local_place(expression: &Expression) -> Option<LocalId> {
    semantic_place(expression)
        .filter(|place| place.projections.is_empty())
        .map(|place| place.root)
}

fn semantic_place(expression: &Expression) -> Option<&Place> {
    match &expression.kind {
        ExpressionKind::Place(place) => Some(place),
        _ => None,
    }
}

fn check_place_indices(
    body: &TypedBody,
    place: &Place,
    states: &mut Vec<State>,
) -> Result<(), Diagnostic> {
    for projection in &place.projections {
        if let super::typed_hir::Projection::Index(index) = projection {
            check_expression(body, &index.index, states)?;
        }
    }
    Ok(())
}

fn require_initialized(state: State, name: &str, span: Span) -> Result<(), Diagnostic> {
    match state {
        State::Initialized => Ok(()),
        State::Uninitialized => Err(error(
            format!("'{name}' is used before initialization"),
            span,
        )),
        State::Moved => Err(error(
            format!("'{name}' is used after its ownership was moved"),
            span,
        )),
        State::MaybeUnavailable => Err(error(
            format!("'{name}' may be uninitialized or moved on this path"),
            span,
        )),
    }
}

fn error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::TYPE_MISMATCH, message, Some(span))
}
