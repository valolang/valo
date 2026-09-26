//! Lower validated scalar function bodies without interpreter-driven resolution.
use super::*;
use crate::TypeKind;
use crate::frontend::semantics::arithmetic::{self, ArithmeticOp};
use crate::frontend::semantics::typed_hir as h;
use crate::runtime::{DiagnosticCode, Span};
use crate::{CaseBranch, CaseCompareOp, CaseItem, ContinueTarget, ElseIfBranch};

/// Lower one function from a Program. Unsupported native constructs are errors;
/// they remain available through the existing source interpreter.
pub fn lower_function_body(
    program: &Program,
    function_index: usize,
) -> Result<h::TypedBody, Diagnostic> {
    validate_snippet(program)?;
    lower_project_function_body(program, function_index, program)
}

/// Project declarations are already validated in their individual source
/// scopes. The combined declaration view supplies resolved call/type targets,
/// while the owner supplies the body-local Option semantics.
pub fn lower_project_function_body(
    program: &Program,
    function_index: usize,
    source: &Program,
) -> Result<h::TypedBody, Diagnostic> {
    let function = program.functions.get(function_index).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::UNKNOWN_NAME,
            "Function index is outside this program",
            None,
        )
    })?;
    check_function(program, function)?;
    lower_callable_body(
        program,
        source,
        h::BodyFunctionId(function_index),
        &function.name,
        &function.params,
        &function.body,
        function.return_type.clone(),
        function.span,
    )
}

/// Native Sub bodies share the same typed control-flow and cleanup lowering as
/// Functions, but have a real Void return rather than a fabricated result.
pub fn lower_project_procedure_body(
    program: &Program,
    procedure_index: usize,
    source: &Program,
) -> Result<h::TypedBody, Diagnostic> {
    let procedure = program.procedures.get(procedure_index).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::UNKNOWN_NAME,
            "Sub index is outside this program",
            None,
        )
    })?;
    if procedure.is_async || !procedure.type_params.is_empty() {
        return Err(unsupported("async or generic Sub bodies", procedure.span));
    }
    for parameter in &procedure.params {
        hir_value_type(program, &parameter.ty, parameter.span)?;
        if parameter.is_optional || parameter.is_param_array {
            return Err(unsupported(
                "Optional or ParamArray parameters",
                parameter.span,
            ));
        }
    }
    lower_callable_body(
        program,
        source,
        h::BodyFunctionId(program.functions.len() + procedure_index),
        &procedure.name,
        &procedure.params,
        &procedure.body,
        TypeName::Void,
        procedure.span,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_callable_body(
    program: &Program,
    source: &Program,
    id: h::BodyFunctionId,
    name: &str,
    params: &[Parameter],
    source_body: &[Stmt],
    return_type: TypeName,
    span: Span,
) -> Result<h::TypedBody, Diagnostic> {
    let types = collect_types(program)?;
    let signatures = collect_signatures(program, &types)?;
    let mut symbols = collect_module_symbols(program, &types, &signatures)?;
    add_parameters(params, &mut symbols)?;
    let mut builder = Builder {
        program,
        types: &types,
        signatures: &signatures,
        symbols,
        options: program_options(source),
        option_compare: source.option_compare,
        owner: program.owners.get(&span).cloned().unwrap_or_default(),
        locals: Vec::new(),
        fields: program
            .types
            .iter()
            .filter(|ty| ty.kind == TypeKind::Structure)
            .flat_map(|ty| {
                ty.fields
                    .iter()
                    .map(|field| (TypeName::User(ty.name.clone()), field))
            })
            .enumerate()
            .map(|(index, (owner, field))| h::ResolvedField {
                id: h::FieldId(index),
                owner,
                ty: if field.array.is_some() {
                    TypeName::Array(Box::new(field.ty.clone()))
                } else {
                    field.ty.clone()
                },
                name: field.name.clone(),
            })
            .chain(
                program
                    .classes
                    .iter()
                    .flat_map(|class| {
                        class.members.iter().flat_map(move |member| {
                            let fields: Vec<&crate::ClassField> = match member {
                                crate::ClassMember::Field(field) => vec![field],
                                crate::ClassMember::Fields(fields) => fields.iter().collect(),
                                _ => Vec::new(),
                            };
                            fields
                                .into_iter()
                                .filter(|field| !field.is_shared)
                                .map(move |field| (TypeName::User(class.name.clone()), field))
                        })
                    })
                    .enumerate()
                    .map(|(offset, (owner, field))| h::ResolvedField {
                        id: h::FieldId(
                            program
                                .types
                                .iter()
                                .filter(|ty| ty.kind == TypeKind::Structure)
                                .map(|ty| ty.fields.len())
                                .sum::<usize>()
                                + offset,
                        ),
                        owner,
                        ty: field.ty.clone().unwrap_or(TypeName::Variant),
                        name: field.name.clone(),
                    }),
            )
            .collect(),
        disposers: program
            .classes
            .iter()
            .flat_map(|class| {
                class.members.iter().filter_map(|member| match member {
                    crate::ClassMember::Sub(method)
                        if method.procedure.name.eq_ignore_ascii_case("Dispose")
                            && method.procedure.params.is_empty()
                            && !method.is_shared =>
                    {
                        Some(TypeName::User(class.name.clone()))
                    }
                    _ => None,
                })
            })
            .enumerate()
            .map(|(index, owner)| h::ResolvedDispose {
                id: h::DisposeMethodId(index),
                owner,
            })
            .collect(),
        scopes: vec![h::Scope {
            id: h::ScopeId(0),
            parent: None,
            locals: Vec::new(),
            cleanup: Vec::new(),
            span,
        }],
        active_scopes: vec![h::ScopeId(0)],
        names: HashMap::new(),
        shadowed_symbols: vec![Vec::new()],
        loops: Vec::new(),
        next_loop: 0,
    };
    for (index, parameter) in params.iter().enumerate() {
        builder.local(
            &parameter.name,
            parameter.ty.clone(),
            Some(index),
            match parameter.mode {
                PassingMode::ByVal => h::LocalStorage::Value,
                PassingMode::ByRef => h::LocalStorage::BorrowedMutable,
                PassingMode::ByRefReadOnly => h::LocalStorage::BorrowedImmutable,
            },
            parameter.span,
        )?;
    }
    let (mut statements, returns) =
        lower_statements(&mut builder, source_body, &return_type, span)?;
    if !returns && return_type == TypeName::Void {
        statements.push(h::Statement::ReturnVoid {
            exited_scopes: vec![h::ScopeId(0)],
            cleanup_chain: builder.cleanup_chain(&[h::ScopeId(0)]),
            span,
        });
    } else if !returns {
        return Err(unsupported(
            "a function with a path that does not Return",
            span,
        ));
    }
    let body = h::TypedBody {
        function: id,
        name: name.to_string(),
        symbol_name: {
            let owner = program.owners.get(&span).map(String::as_str).unwrap_or("");
            let modes = params
                .iter()
                .map(|param| format!("{:?}:{}", param.mode, param.ty.display_name()))
                .collect::<Vec<_>>()
                .join(",");
            format!("{owner}::{name}({modes})->{}", return_type.display_name()).to_ascii_lowercase()
        },
        return_type,
        locals: builder.locals,
        structures: program
            .types
            .iter()
            .filter(|decl| decl.kind == TypeKind::Structure)
            .map(|decl| TypeName::User(decl.name.clone()))
            .collect(),
        classes: program
            .classes
            .iter()
            .map(|class| TypeName::User(class.name.clone()))
            .collect(),
        fields: builder.fields,
        disposers: builder.disposers,
        scopes: builder.scopes,
        root_scope: h::ScopeId(0),
        statements,
        span,
    };
    crate::frontend::semantics::verify_hir::verify_body(&body)?;
    crate::frontend::semantics::ownership::check_body(&body)?;
    Ok(body)
}

fn lower_statements(
    builder: &mut Builder<'_>,
    source: &[Stmt],
    return_type: &TypeName,
    fallback_span: Span,
) -> Result<(Vec<h::Statement>, bool), Diagnostic> {
    let mut statements = Vec::new();
    let mut returns = false;
    for statement in source {
        if returns {
            return Err(unsupported(
                "unreachable statements after Return",
                statement_span(statement, fallback_span),
            ));
        }
        match statement {
            Stmt::Dim {
                name,
                ty: Some(ty),
                array: None,
                as_new: true,
                new_args,
                initializer: None,
                collection_initializer: None,
                member_initializer: None,
                span,
                ..
            } => {
                if !new_args.is_empty() {
                    return Err(unsupported("native Class constructor arguments", *span));
                }
                if !is_collection(ty) {
                    validate_native_class_creation(builder.program, ty, *span)?;
                }
                let id = builder.local(name, ty.clone(), None, h::LocalStorage::Value, *span)?;
                statements.push(h::Statement::Initialize {
                    target: id,
                    value: h::Expression {
                        kind: if is_collection(ty) {
                            h::ExpressionKind::NewCollection
                        } else {
                            h::ExpressionKind::NewClass(ty.clone())
                        },
                        ty: ty.clone(),
                        category: h::ValueCategory::Value,
                        span: *span,
                    },
                    span: *span,
                });
            }
            Stmt::Dim {
                name,
                ty,
                array: None,
                as_new: false,
                initializer,
                collection_initializer: None,
                member_initializer: None,
                span,
                ..
            } => {
                let value = if let Some(expr) = initializer {
                    let value = if matches!(expr.kind, ExprKind::Nothing) {
                        null_reference(
                            builder.program,
                            ty.as_ref()
                                .ok_or_else(|| cannot_infer_variable(name, *span))?,
                            expr.span,
                        )?
                    } else {
                        builder.expression(expr)?
                    };
                    if let Some(ty) = ty {
                        convert(value, ty, h::Conversion::NumericChecked)?
                    } else {
                        value
                    }
                } else {
                    let ty = ty
                        .as_ref()
                        .ok_or_else(|| cannot_infer_variable(name, *span))?;
                    hir_value_type(builder.program, ty, *span)?;
                    h::Expression {
                        kind: h::ExpressionKind::Constant(match ty {
                            TypeName::Boolean => h::Constant::Boolean(false),
                            TypeName::Single => h::Constant::Single(0.0),
                            TypeName::Double => h::Constant::Double(0.0),
                            TypeName::String => h::Constant::String(String::new()),
                            TypeName::Variant => h::Constant::NullReference,
                            TypeName::User(_) | TypeName::Tuple(_) => h::Constant::ZeroAggregate,
                            _ => h::Constant::Integer(0),
                        }),
                        ty: ty.clone(),
                        category: h::ValueCategory::Value,
                        span: *span,
                    }
                };
                let id =
                    builder.local(name, value.ty.clone(), None, h::LocalStorage::Value, *span)?;
                statements.push(h::Statement::Initialize {
                    target: id,
                    value,
                    span: *span,
                });
            }
            Stmt::Dim {
                name,
                ty: Some(element_type),
                array: Some(ArrayDecl::Fixed(bounds)),
                as_new: false,
                initializer: None,
                collection_initializer: None,
                member_initializer: None,
                span,
                ..
            } if bounds.len() == 1 && bounds[0].lower == 0 => {
                hir_value_type(builder.program, element_type, *span)?;
                let bound = bounds[0];
                if bound.upper < 0 {
                    return Err(unsupported("negative fixed-array upper bounds", *span));
                }
                let ty = TypeName::Array(Box::new(element_type.clone()));
                let id = builder.local(name, ty.clone(), None, h::LocalStorage::Value, *span)?;
                statements.push(h::Statement::Initialize {
                    target: id,
                    value: h::Expression {
                        kind: h::ExpressionKind::ArrayInit {
                            lower: bound.lower,
                            upper: bound.upper,
                        },
                        ty,
                        category: h::ValueCategory::Value,
                        span: *span,
                    },
                    span: *span,
                });
            }
            Stmt::Assign {
                target:
                    AssignTarget::Variable {
                        name,
                        span: target_span,
                    },
                expr,
                span,
            } => {
                let id = builder.lookup(name, *target_span)?;
                let target = builder.place(id, *target_span);
                let source = if matches!(expr.kind, ExprKind::Nothing) {
                    null_reference(builder.program, &target.ty, expr.span)?
                } else {
                    builder.expression(expr)?
                };
                let value = convert(source, &target.ty, h::Conversion::NumericChecked)?;
                statements.push(h::Statement::Store {
                    target: Box::new(target),
                    value,
                    span: *span,
                });
            }
            Stmt::Assign {
                target:
                    AssignTarget::ArrayElement {
                        name,
                        indices,
                        span: target_span,
                    },
                expr,
                span,
            } => {
                let base = builder.place(builder.lookup(name, *target_span)?, *target_span);
                let target = builder.indexed_place(base, indices, *target_span)?;
                let value = convert(
                    builder.expression(expr)?,
                    &target.ty,
                    h::Conversion::NumericChecked,
                )?;
                statements.push(h::Statement::Store {
                    target: Box::new(target),
                    value,
                    span: *span,
                });
            }
            Stmt::Assign {
                target:
                    AssignTarget::Member {
                        object,
                        field,
                        span: target_span,
                    },
                expr,
                span,
            } => {
                let target =
                    builder.field_place(builder.source_place(object)?, field, *target_span)?;
                let value = convert(
                    builder.expression(expr)?,
                    &target.ty,
                    h::Conversion::NumericChecked,
                )?;
                statements.push(h::Statement::Store {
                    target: Box::new(target),
                    value,
                    span: *span,
                });
            }
            Stmt::Return { expr, span } => {
                let source = if matches!(expr.kind, ExprKind::Nothing) {
                    null_reference(builder.program, return_type, expr.span)?
                } else {
                    builder.expression(expr)?
                };
                let value = convert(source, return_type, h::Conversion::NumericChecked)?;
                let exited_scopes: Vec<_> = builder.active_scopes.iter().rev().copied().collect();
                statements.push(h::Statement::Return {
                    value,
                    cleanup_chain: builder.cleanup_chain(&exited_scopes),
                    exited_scopes,
                    span: *span,
                });
                returns = true;
            }
            Stmt::Exit {
                target: ExitTarget::Sub,
                span,
            } if *return_type == TypeName::Void => {
                let exited_scopes: Vec<_> = builder.active_scopes.iter().rev().copied().collect();
                statements.push(h::Statement::ReturnVoid {
                    cleanup_chain: builder.cleanup_chain(&exited_scopes),
                    exited_scopes,
                    span: *span,
                });
                returns = true;
            }
            Stmt::SubCall { name, args, span } => {
                let resolved_name = if builder.signatures.subs.contains_key(&key(name)) {
                    name.clone()
                } else {
                    format!("{}.{}", builder.owner, name)
                };
                let candidates = builder
                    .signatures
                    .subs
                    .get(&key(&resolved_name))
                    .ok_or_else(|| unsupported("external or built-in Sub calls", *span))?;
                let selected = resolve_overload(
                    "Sub",
                    name,
                    candidates,
                    args,
                    *span,
                    ExprValidation::new(
                        &builder.symbols,
                        builder.types,
                        builder.signatures,
                        &Context::Sub { is_async: false },
                        builder.options,
                    ),
                )?;
                let (index, procedure) = builder
                    .program
                    .procedures
                    .iter()
                    .enumerate()
                    .find(|(_, procedure)| {
                        key(&procedure.name) == key(&resolved_name)
                            && procedure.params.len() == selected.params.len()
                            && procedure
                                .params
                                .iter()
                                .zip(&selected.params)
                                .all(|(a, b)| a.ty.same_type(&b.ty) && a.mode == b.mode)
                    })
                    .ok_or_else(|| unsupported("external or specialized Sub calls", *span))?;
                if args.len() != procedure.params.len() {
                    return Err(unsupported("omitted Sub arguments", *span));
                }
                let arguments = builder.call_arguments(args, &procedure.params)?;
                statements.push(h::Statement::CallSub {
                    function: h::BodyFunctionId(builder.program.functions.len() + index),
                    signature: h::CallSignature {
                        parameter_types: procedure.params.iter().map(|p| p.ty.clone()).collect(),
                        parameter_modes: arguments.iter().map(|arg| arg.mode).collect(),
                        return_type: TypeName::Void,
                    },
                    arguments,
                    span: *span,
                });
            }
            Stmt::MemberSubCall {
                object,
                method,
                args,
                span,
            } => {
                let collection = builder.expression(object)?;
                if !is_collection(&collection.ty) {
                    return Err(unsupported("native instance method calls", *span));
                }
                if method.eq_ignore_ascii_case("Add") {
                    if args.is_empty() || args.len() > 4 {
                        return Err(unsupported("Collection.Add argument count", *span));
                    }
                    if args
                        .get(1)
                        .is_some_and(|arg| !matches!(arg.kind, ExprKind::Missing))
                    {
                        return Err(unsupported("native Collection String keys", args[1].span));
                    }
                    if args
                        .get(3)
                        .is_some_and(|arg| !matches!(arg.kind, ExprKind::Missing))
                    {
                        return Err(unsupported(
                            "native Collection After positioning",
                            args[3].span,
                        ));
                    }
                    let item = convert(
                        builder.expression(&args[0])?,
                        &TypeName::Variant,
                        h::Conversion::NumericChecked,
                    )?;
                    let before = args
                        .get(2)
                        .filter(|arg| !matches!(arg.kind, ExprKind::Missing))
                        .map(|arg| {
                            convert(
                                builder.expression(arg)?,
                                &TypeName::Int64,
                                h::Conversion::NumericChecked,
                            )
                        })
                        .transpose()?;
                    statements.push(h::Statement::CollectionAdd {
                        collection,
                        item,
                        before: Box::new(before),
                        span: *span,
                    });
                } else if method.eq_ignore_ascii_case("Remove") && args.len() == 1 {
                    let index = convert(
                        builder.expression(&args[0])?,
                        &TypeName::Int64,
                        h::Conversion::NumericChecked,
                    )?;
                    statements.push(h::Statement::CollectionRemove {
                        collection,
                        index,
                        span: *span,
                    });
                } else {
                    return Err(unsupported("this native Collection operation", *span));
                }
            }
            Stmt::TryCatch {
                try_body,
                catch_block,
                finally_body,
                span,
            } => {
                // Reserve the sibling Finally scope before the Try body so
                // transfers in that body can name its handler immediately.
                let finally_scope = finally_body.as_ref().map(|_| builder.reserve_scope(*span));
                let catch_scope = catch_block.as_ref().map(|_| builder.reserve_scope(*span));
                let try_scope = builder.enter_scope(*span);
                if let Some(finally_scope) = finally_scope {
                    builder.scopes[try_scope.0]
                        .cleanup
                        .push(h::ScopeCleanup::FinallyRegion { finally_scope });
                }
                let (try_body, try_exits) =
                    lower_statements(builder, try_body, return_type, *span)?;
                builder.leave_scope();
                let (catch_local, catch_body, catch_exits) =
                    if let (Some(catch), Some(scope)) = (catch_block, catch_scope) {
                        builder.activate_scope(scope);
                        if let Some(finally_scope) = finally_scope {
                            builder.scopes[scope.0]
                                .cleanup
                                .push(h::ScopeCleanup::FinallyRegion { finally_scope });
                        }
                        let catch_local = if let Some(name) = &catch.variable {
                            let id = builder.local(
                                name,
                                TypeName::User("Error".into()),
                                None,
                                h::LocalStorage::Value,
                                catch.span,
                            )?;
                            builder.locals[id.0].initial_state = h::InitialState::Initialized;
                            Some(id)
                        } else {
                            None
                        };
                        let (body, exits) =
                            lower_statements(builder, &catch.body, return_type, catch.span)?;
                        builder.leave_scope();
                        (catch_local, body, exits)
                    } else {
                        (None, Vec::new(), false)
                    };
                let finally_body = if let (Some(source), Some(scope)) =
                    (finally_body, finally_scope)
                {
                    builder.activate_scope(scope);
                    let (body, exits) = lower_statements(builder, source, return_type, *span)?;
                    builder.leave_scope();
                    if exits {
                        return Err(unsupported("a transfer out of Finally in typed HIR", *span));
                    }
                    body
                } else {
                    Vec::new()
                };
                if let Some(catch_scope) = catch_scope {
                    statements.push(h::Statement::TryCatch {
                        try_scope,
                        try_body,
                        catch_scope,
                        catch_local,
                        catch_body,
                        finally_scope,
                        finally_body,
                        span: *span,
                    });
                    returns = try_exits && catch_exits;
                } else if let Some(finally_scope) = finally_scope {
                    statements.push(h::Statement::TryFinally {
                        try_scope,
                        try_body,
                        finally_scope,
                        finally_body,
                        span: *span,
                    });
                    returns = try_exits;
                } else {
                    return Err(unsupported("Try without Catch or Finally", *span));
                }
            }
            Stmt::Using {
                resource,
                body,
                span,
            } => {
                let (resource_name, resource_type, initializer) = match resource {
                    crate::UsingResource::Target(target) => {
                        let place = builder.source_place(target)?;
                        let name = format!("@using{}", builder.scopes.len());
                        let ty = place.ty.clone();
                        (
                            name,
                            ty.clone(),
                            h::Expression {
                                kind: h::ExpressionKind::Load(Box::new(place)),
                                ty,
                                category: h::ValueCategory::Value,
                                span: *span,
                            },
                        )
                    }
                    crate::UsingResource::Declaration(declaration) => {
                        if declaration.as_new
                            || declaration.array.is_some()
                            || declaration.collection_initializer.is_some()
                            || declaration.member_initializer.is_some()
                        {
                            return Err(unsupported(
                                "Using declarations requiring construction or collection initialization in typed HIR",
                                declaration.span,
                            ));
                        }
                        let initializer = declaration.initializer.as_ref().ok_or_else(|| {
                            unsupported(
                                "Using declaration without an initializer",
                                declaration.span,
                            )
                        })?;
                        let value = builder.expression(initializer)?;
                        let ty = declaration.ty.as_ref().ok_or_else(|| {
                            unsupported(
                                "Using declaration without a resolved type",
                                declaration.span,
                            )
                        })?;
                        let value = convert(value, ty, h::Conversion::NumericChecked)?;
                        (declaration.name.clone(), ty.clone(), value)
                    }
                };
                let method = builder
                    .disposers
                    .iter()
                    .find(|method| method.owner.same_type(&resource_type))
                    .ok_or_else(|| unsupported("Using without a resolved Dispose method", *span))?
                    .id;
                let body_scope = builder.enter_scope(*span);
                let resource = builder.local(
                    &resource_name,
                    resource_type,
                    None,
                    h::LocalStorage::Value,
                    *span,
                )?;
                builder.scopes[body_scope.0]
                    .cleanup
                    .push(h::ScopeCleanup::ExplicitDispose { resource, method });
                let initialize = h::Statement::Initialize {
                    target: resource,
                    value: initializer,
                    span: *span,
                };
                let (mut lowered, body_exits) =
                    lower_statements(builder, body, return_type, *span)?;
                lowered.insert(0, initialize);
                builder.leave_scope();
                statements.push(h::Statement::UsingDispose {
                    resource,
                    method,
                    body_scope,
                    body: lowered,
                    span: *span,
                });
                returns = body_exits;
            }
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                span,
            } => {
                let condition = boolean_condition(builder.expression(condition)?)?;
                let (then_scope, then_body, then_returns) =
                    lower_scoped(builder, then_body, return_type, *span)?;
                let (else_scope, else_body, else_returns) =
                    lower_else_chain(builder, elseif_branches, else_body, return_type, *span)?;
                statements.push(h::Statement::If {
                    condition,
                    then_scope,
                    then_body,
                    else_scope,
                    else_body,
                    span: *span,
                });
                returns = then_returns && else_returns;
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                let condition = boolean_condition(builder.expression(condition)?)?;
                let (id, body_scope, body) =
                    lower_loop_body(builder, ContinueTarget::While, body, return_type, *span)?;
                statements.push(h::Statement::While {
                    id,
                    condition,
                    body_scope,
                    body,
                    span: *span,
                });
            }
            Stmt::DoLoop {
                condition,
                body,
                span,
            } => {
                let condition = match condition {
                    DoLoopCondition::Infinite => h::DoCondition::Infinite,
                    DoLoopCondition::PreWhile(e) => {
                        h::DoCondition::PreWhile(boolean_condition(builder.expression(e)?)?)
                    }
                    DoLoopCondition::PreUntil(e) => {
                        h::DoCondition::PreUntil(boolean_condition(builder.expression(e)?)?)
                    }
                    DoLoopCondition::PostWhile(e) => {
                        h::DoCondition::PostWhile(boolean_condition(builder.expression(e)?)?)
                    }
                    DoLoopCondition::PostUntil(e) => {
                        h::DoCondition::PostUntil(boolean_condition(builder.expression(e)?)?)
                    }
                };
                let (id, body_scope, body) =
                    lower_loop_body(builder, ContinueTarget::Do, body, return_type, *span)?;
                statements.push(h::Statement::Do {
                    id,
                    condition,
                    body_scope,
                    body,
                    span: *span,
                });
            }
            Stmt::For {
                variable,
                start,
                end,
                step,
                body,
                span,
                ..
            } => {
                let variable = builder.lookup(variable, *span)?;
                let ty = builder.locals[variable.0].ty.clone();
                if !ty.is_integral() {
                    return Err(unsupported("non-integral For loop variables", *span));
                }
                let start = convert(
                    builder.expression(start)?,
                    &ty,
                    h::Conversion::NumericChecked,
                )?;
                let end = convert(builder.expression(end)?, &ty, h::Conversion::NumericChecked)?;
                let step = if let Some(step) = step {
                    convert(
                        builder.expression(step)?,
                        &ty,
                        h::Conversion::NumericChecked,
                    )?
                } else {
                    h::Expression {
                        kind: h::ExpressionKind::Constant(h::Constant::Integer(1)),
                        ty,
                        category: h::ValueCategory::Value,
                        span: *span,
                    }
                };
                let (id, body_scope, body) =
                    lower_loop_body(builder, ContinueTarget::For, body, return_type, *span)?;
                statements.push(h::Statement::For {
                    id,
                    variable,
                    start: Box::new(start),
                    end: Box::new(end),
                    step: Box::new(step),
                    body_scope,
                    body,
                    span: *span,
                });
            }
            Stmt::ForEach {
                variable,
                iterable,
                body,
                span,
                ..
            } => {
                let variable = builder.lookup(variable, *span)?;
                let iterable = builder.expression(iterable)?;
                let element_type = match &iterable.ty {
                    TypeName::Array(element) => (**element).clone(),
                    ty if is_collection(ty) => TypeName::Variant,
                    _ => {
                        return Err(unsupported(
                            "For Each over non-array/Collection values",
                            iterable.span,
                        ));
                    }
                };
                if !builder.locals[variable.0].ty.same_type(&element_type) {
                    return Err(unsupported("For Each element conversions", *span));
                }
                let (id, body_scope, body) =
                    lower_loop_body(builder, ContinueTarget::For, body, return_type, *span)?;
                statements.push(h::Statement::ForEach {
                    id,
                    variable,
                    iterable,
                    element_type,
                    body_scope,
                    body,
                    span: *span,
                });
            }
            Stmt::Exit { target, span }
                if matches!(target, ExitTarget::For | ExitTarget::While | ExitTarget::Do) =>
            {
                let frame = builder.find_loop(exit_loop_kind(*target), *span)?;
                let exited_scopes = builder.exited_scopes_to(frame.parent_scope);
                statements.push(h::Statement::ExitLoop {
                    loop_id: frame.id,
                    cleanup_chain: builder.cleanup_chain(&exited_scopes),
                    exited_scopes,
                    span: *span,
                });
                returns = true;
            }
            Stmt::Continue { target, span } => {
                let frame = builder.find_loop(*target, *span)?;
                let exited_scopes = builder.exited_scopes_to(frame.parent_scope);
                statements.push(h::Statement::ContinueLoop {
                    loop_id: frame.id,
                    cleanup_chain: builder.cleanup_chain(&exited_scopes),
                    exited_scopes,
                    span: *span,
                });
                returns = true;
            }
            Stmt::SelectCase {
                subject,
                branches,
                else_body,
                span,
            } => {
                let value = builder.expression(subject)?;
                // The selector is evaluated exactly once. This first lowering
                // supports only trivial scalar values: a managed selector
                // needs a distinct lexical cleanup scope.
                if !value.ty.is_integral()
                    && !matches!(
                        value.ty,
                        TypeName::Boolean | TypeName::Single | TypeName::Double
                    )
                {
                    return Err(unsupported(
                        "native Select Case over this selector type",
                        subject.span,
                    ));
                }
                let hidden = format!("@select{}", builder.locals.len());
                let id = builder.local(
                    &hidden,
                    value.ty.clone(),
                    None,
                    h::LocalStorage::Value,
                    *span,
                )?;
                statements.push(h::Statement::Initialize {
                    target: id,
                    value,
                    span: *span,
                });
                let decision = select_decision_tree(&hidden, branches, else_body, *span);
                let (lowered, exits) = lower_statements(builder, &decision, return_type, *span)?;
                statements.extend(lowered);
                returns = exits;
            }
            _ => {
                return Err(unsupported(
                    "this statement in a scalar function body",
                    statement_span(statement, fallback_span),
                ));
            }
        }
    }
    Ok((statements, returns))
}

/// Desugar source Case tests into ordinary typed If branches. The hidden
/// selector local is initialized above, so repeated tests never reevaluate
/// the selector. Each branch body is lowered independently into its own HIR
/// scope; MIR/LLVM only receive resolved control flow and comparison values.
fn select_decision_tree(
    selector: &str,
    branches: &[CaseBranch],
    else_body: &[Stmt],
    span: Span,
) -> Vec<Stmt> {
    let mut continuation = else_body.to_vec();
    let mut range_number = 0;
    for branch in branches.iter().rev() {
        for item in branch.items.iter().rev() {
            let test = |op, right: &Expr| Expr {
                kind: ExprKind::Binary {
                    left: Box::new(Expr {
                        kind: ExprKind::Variable(selector.to_string()),
                        span: right.span,
                    }),
                    op,
                    right: Box::new(right.clone()),
                },
                span: right.span,
            };
            let choose = |condition, yes: Vec<Stmt>, no: Vec<Stmt>| Stmt::If {
                condition,
                then_body: yes,
                elseif_branches: Vec::new(),
                else_body: no,
                span,
            };
            continuation = match item {
                CaseItem::Value(value) => vec![choose(
                    test(crate::BinaryOp::Equal, value),
                    branch.body.clone(),
                    continuation,
                )],
                CaseItem::Compare { op, expr } => {
                    let operation = match op {
                        CaseCompareOp::Equal => crate::BinaryOp::Equal,
                        CaseCompareOp::NotEqual => crate::BinaryOp::NotEqual,
                        CaseCompareOp::Less => crate::BinaryOp::Less,
                        CaseCompareOp::Greater => crate::BinaryOp::Greater,
                        CaseCompareOp::LessEqual => crate::BinaryOp::LessEqual,
                        CaseCompareOp::GreaterEqual => crate::BinaryOp::GreaterEqual,
                    };
                    vec![choose(
                        test(operation, expr),
                        branch.body.clone(),
                        continuation,
                    )]
                }
                CaseItem::Range { start, end } => {
                    // The interpreter evaluates both range endpoints before
                    // testing either bound, even when the lower bound fails.
                    // Capture each endpoint once before the branch.
                    let number = range_number;
                    range_number += 1;
                    let lower_name = format!("{selector}@range{number}lower");
                    let upper_name = format!("{selector}@range{number}upper");
                    let capture = |name: &str, value: &Expr| Stmt::Dim {
                        name: name.to_string(),
                        ty: None,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        initializer: Some(value.clone()),
                        collection_initializer: None,
                        member_initializer: None,
                        span: value.span,
                    };
                    let lower = Expr {
                        kind: ExprKind::Variable(lower_name.clone()),
                        span: start.span,
                    };
                    let upper = Expr {
                        kind: ExprKind::Variable(upper_name.clone()),
                        span: end.span,
                    };
                    vec![
                        capture(&lower_name, start),
                        capture(&upper_name, end),
                        choose(
                            test(crate::BinaryOp::GreaterEqual, &lower),
                            vec![choose(
                                test(crate::BinaryOp::LessEqual, &upper),
                                branch.body.clone(),
                                continuation.clone(),
                            )],
                            continuation,
                        ),
                    ]
                }
            };
        }
    }
    continuation
}

fn lower_scoped(
    builder: &mut Builder<'_>,
    source: &[Stmt],
    return_type: &TypeName,
    span: Span,
) -> Result<(h::ScopeId, Vec<h::Statement>, bool), Diagnostic> {
    let scope = builder.enter_scope(span);
    let (body, terminated) = lower_statements(builder, source, return_type, span)?;
    builder.leave_scope();
    Ok((scope, body, terminated))
}

fn lower_else_chain(
    builder: &mut Builder<'_>,
    branches: &[ElseIfBranch],
    else_body: &[Stmt],
    return_type: &TypeName,
    span: Span,
) -> Result<(h::ScopeId, Vec<h::Statement>, bool), Diagnostic> {
    if let Some((first, rest)) = branches.split_first() {
        let scope = builder.enter_scope(first.condition.span);
        let condition = boolean_condition(builder.expression(&first.condition)?)?;
        let (then_scope, then_body, then_returns) =
            lower_scoped(builder, &first.body, return_type, first.condition.span)?;
        let (else_scope, else_body, else_returns) =
            lower_else_chain(builder, rest, else_body, return_type, span)?;
        builder.leave_scope();
        Ok((
            scope,
            vec![h::Statement::If {
                condition,
                then_scope,
                then_body,
                else_scope,
                else_body,
                span: first.condition.span,
            }],
            then_returns && else_returns,
        ))
    } else {
        lower_scoped(builder, else_body, return_type, span)
    }
}

fn lower_loop_body(
    builder: &mut Builder<'_>,
    kind: ContinueTarget,
    source: &[Stmt],
    return_type: &TypeName,
    span: Span,
) -> Result<(h::LoopId, h::ScopeId, Vec<h::Statement>), Diagnostic> {
    let id = h::LoopId(builder.next_loop);
    builder.next_loop += 1;
    builder.loops.push(LoopFrame {
        id,
        kind,
        parent_scope: builder.current_scope(),
    });
    let (scope, body, _) = lower_scoped(builder, source, return_type, span)?;
    builder.loops.pop();
    Ok((id, scope, body))
}

fn exit_loop_kind(kind: ExitTarget) -> ContinueTarget {
    match kind {
        ExitTarget::For => ContinueTarget::For,
        ExitTarget::While => ContinueTarget::While,
        ExitTarget::Do => ContinueTarget::Do,
        _ => unreachable!("only loop exits call this helper"),
    }
}

fn boolean_condition(condition: h::Expression) -> Result<h::Expression, Diagnostic> {
    if condition.ty.same_type(&TypeName::Boolean) {
        Ok(condition)
    } else {
        Err(unsupported("non-Boolean branch conditions", condition.span))
    }
}

fn statement_span(statement: &Stmt, fallback: Span) -> Span {
    match statement {
        Stmt::Dim { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::If { span, .. } => *span,
        _ => fallback,
    }
}

fn unsupported(feature: &str, span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::TYPE_MISMATCH,
        format!("Typed HIR lowering does not yet support {feature}"), Some(span))
        .with_help("This construct can still use the source interpreter; native lowering requires additional implementation")
}
fn scalar(ty: &TypeName, span: Span) -> Result<(), Diagnostic> {
    if ty.is_integral()
        || matches!(
            ty,
            TypeName::Single
                | TypeName::Double
                | TypeName::Boolean
                | TypeName::String
                | TypeName::Variant
        )
    {
        Ok(())
    } else {
        Err(unsupported(&format!("type '{}'", ty.display_name()), span))
    }
}
fn is_collection(ty: &TypeName) -> bool {
    matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
}
fn hir_value_type(program: &Program, ty: &TypeName, span: Span) -> Result<(), Diagnostic> {
    match ty {
        TypeName::User(name)
            if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION) =>
        {
            Ok(())
        }
        TypeName::User(name) if name.eq_ignore_ascii_case("Error") => Ok(()),
        TypeName::Array(element) => hir_value_type(program, element, span),
        TypeName::Tuple(elements) => {
            for element in elements {
                hir_value_type(program, &element.ty, span)?;
            }
            Ok(())
        }
        TypeName::User(name)
            if program.types.iter().any(|decl| {
                decl.kind == TypeKind::Structure && decl.name.eq_ignore_ascii_case(name)
            }) =>
        {
            Ok(())
        }
        TypeName::User(name)
            if program
                .classes
                .iter()
                .any(|decl| decl.name.eq_ignore_ascii_case(name)) =>
        {
            Ok(())
        }
        _ => scalar(ty, span),
    }
}

fn validate_native_class_creation(
    program: &Program,
    ty: &TypeName,
    span: Span,
) -> Result<(), Diagnostic> {
    let TypeName::User(name) = ty else {
        return Err(unsupported("native creation of this type", span));
    };
    let class = program
        .classes
        .iter()
        .find(|class| class.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| unsupported("native creation of an unresolved Class", span))?;
    if class.base_class.is_some() || class.inheritance == crate::ClassInheritance::MustInherit {
        return Err(unsupported(
            "native Class inheritance or abstract allocation",
            span,
        ));
    }
    if class.members.iter().any(|member| match member {
        crate::ClassMember::Sub(method) => {
            method.procedure.name.eq_ignore_ascii_case("Initialize")
                || method.procedure.name.eq_ignore_ascii_case("New")
                || method.procedure.name.eq_ignore_ascii_case("Terminate")
        }
        crate::ClassMember::Field(field) => {
            field.initializer.is_some() || field.as_new || field.collection_initializer.is_some()
        }
        crate::ClassMember::Fields(fields) => fields.iter().any(|field| {
            field.initializer.is_some() || field.as_new || field.collection_initializer.is_some()
        }),
        _ => false,
    }) {
        return Err(unsupported(
            "native Class constructor, Terminate finalizer, or field initializers",
            span,
        ));
    }
    Ok(())
}

fn null_reference(
    program: &Program,
    ty: &TypeName,
    span: Span,
) -> Result<h::Expression, Diagnostic> {
    if *ty == TypeName::Variant
        || matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
    {
        return Ok(h::Expression {
            kind: h::ExpressionKind::Constant(h::Constant::NullReference),
            ty: ty.clone(),
            category: h::ValueCategory::Value,
            span,
        });
    }
    let TypeName::User(name) = ty else {
        return Err(unsupported("Nothing for a non-Class native value", span));
    };
    if !program
        .classes
        .iter()
        .any(|class| class.name.eq_ignore_ascii_case(name))
    {
        return Err(unsupported("Nothing for an unresolved native Class", span));
    }
    Ok(h::Expression {
        kind: h::ExpressionKind::Constant(h::Constant::NullReference),
        ty: ty.clone(),
        category: h::ValueCategory::Value,
        span,
    })
}
fn check_function(program: &Program, function: &Function) -> Result<(), Diagnostic> {
    if function.is_async || function.is_iterator || !function.type_params.is_empty() {
        return Err(unsupported(
            "async, iterator or generic functions",
            function.span,
        ));
    }
    hir_value_type(program, &function.return_type, function.span)?;
    for parameter in &function.params {
        hir_value_type(program, &parameter.ty, parameter.span)?;
        if parameter.is_optional || parameter.is_param_array {
            return Err(unsupported(
                "Optional or ParamArray parameters",
                parameter.span,
            ));
        }
    }
    Ok(())
}
fn convert(
    value: h::Expression,
    target: &TypeName,
    conversion: h::Conversion,
) -> Result<h::Expression, Diagnostic> {
    if value.ty.same_type(target) {
        return Ok(value);
    }
    if *target == TypeName::Variant {
        return Ok(h::Expression {
            kind: h::ExpressionKind::BoxDynamic(Box::new(value.clone())),
            ty: TypeName::Variant,
            category: h::ValueCategory::Value,
            span: value.span,
        });
    }
    if value.ty == TypeName::Variant {
        return Ok(h::Expression {
            kind: h::ExpressionKind::UnboxDynamic {
                value: Box::new(value.clone()),
                target: target.clone(),
            },
            ty: target.clone(),
            category: h::ValueCategory::Value,
            span: value.span,
        });
    }
    scalar(target, value.span)?;
    if matches!(target, TypeName::Boolean) || matches!(value.ty, TypeName::Boolean) {
        return Err(unsupported(
            "implicit Boolean/numeric conversions",
            value.span,
        ));
    }
    let span = value.span;
    Ok(h::Expression {
        kind: h::ExpressionKind::Convert {
            value: Box::new(value),
            conversion,
        },
        ty: target.clone(),
        category: h::ValueCategory::Value,
        span,
    })
}

struct Builder<'a> {
    program: &'a Program,
    types: &'a TypeRegistry,
    signatures: &'a Signatures,
    symbols: HashMap<String, VarType>,
    options: Options,
    option_compare: crate::OptionCompare,
    owner: String,
    locals: Vec<h::Local>,
    fields: Vec<h::ResolvedField>,
    disposers: Vec<h::ResolvedDispose>,
    scopes: Vec<h::Scope>,
    active_scopes: Vec<h::ScopeId>,
    names: HashMap<String, Vec<h::LocalId>>,
    shadowed_symbols: Vec<Vec<(String, Option<VarType>)>>,
    loops: Vec<LoopFrame>,
    next_loop: usize,
}
#[derive(Clone, Copy)]
struct LoopFrame {
    id: h::LoopId,
    kind: ContinueTarget,
    parent_scope: h::ScopeId,
}
impl Builder<'_> {
    fn call_arguments(
        &self,
        args: &[Expr],
        params: &[Parameter],
    ) -> Result<Vec<h::CallArgument>, Diagnostic> {
        let mut arguments = Vec::new();
        for (arg, param) in args.iter().zip(params) {
            let value = if param.mode != PassingMode::ByVal {
                let place = self.source_place(arg)?;
                if !place.ty.same_type(&param.ty) {
                    return Err(unsupported(
                        "ByRef conversions requiring temporaries",
                        arg.span,
                    ));
                }
                h::Expression {
                    ty: place.ty.clone(),
                    span: arg.span,
                    category: if param.mode == PassingMode::ByRefReadOnly {
                        h::ValueCategory::ImmutableReference
                    } else {
                        h::ValueCategory::MutableReference
                    },
                    kind: if param.mode == PassingMode::ByRefReadOnly {
                        h::ExpressionKind::BorrowImmutable(Box::new(place))
                    } else {
                        h::ExpressionKind::BorrowMutable(Box::new(place))
                    },
                }
            } else {
                convert(
                    self.expression(arg)?,
                    &param.ty,
                    h::Conversion::NumericChecked,
                )?
            };
            arguments.push(h::CallArgument {
                mode: match param.mode {
                    PassingMode::ByVal => h::ArgumentMode::ByVal,
                    PassingMode::ByRef => h::ArgumentMode::BorrowMutable,
                    PassingMode::ByRefReadOnly => h::ArgumentMode::BorrowImmutable,
                },
                value,
            });
        }
        Ok(arguments)
    }
    fn current_scope(&self) -> h::ScopeId {
        *self.active_scopes.last().expect("root scope exists")
    }
    fn enter_scope(&mut self, span: Span) -> h::ScopeId {
        let id = self.reserve_scope(span);
        self.activate_scope(id);
        id
    }
    fn reserve_scope(&mut self, span: Span) -> h::ScopeId {
        let id = h::ScopeId(self.scopes.len());
        self.scopes.push(h::Scope {
            id,
            parent: Some(self.current_scope()),
            locals: Vec::new(),
            cleanup: Vec::new(),
            span,
        });
        self.shadowed_symbols.push(Vec::new());
        id
    }
    fn activate_scope(&mut self, id: h::ScopeId) {
        debug_assert_eq!(self.scopes[id.0].parent, Some(self.current_scope()));
        self.active_scopes.push(id);
    }
    fn cleanup_chain(&self, exited_scopes: &[h::ScopeId]) -> Vec<h::CleanupStep> {
        exited_scopes
            .iter()
            .flat_map(|scope| {
                self.scopes[scope.0]
                    .cleanup
                    .iter()
                    .rev()
                    .cloned()
                    .map(|action| h::CleanupStep {
                        owner: *scope,
                        action,
                    })
            })
            .collect()
    }
    fn leave_scope(&mut self) {
        let scope = self.active_scopes.pop().expect("scope exists");
        for local in self.scopes[scope.0].locals.iter().rev() {
            let name = key(&self.locals[local.0].name);
            let stack = self.names.get_mut(&name).expect("local binding exists");
            stack.pop();
            if stack.is_empty() {
                self.names.remove(&name);
            }
        }
        for (name, previous) in std::mem::take(&mut self.shadowed_symbols[scope.0]) {
            if let Some(previous) = previous {
                self.symbols.insert(name, previous);
            } else {
                self.symbols.remove(&name);
            }
        }
    }
    fn exited_scopes_to(&self, parent: h::ScopeId) -> Vec<h::ScopeId> {
        self.active_scopes
            .iter()
            .rev()
            .copied()
            .take_while(|id| *id != parent)
            .collect()
    }
    fn find_loop(&self, kind: ContinueTarget, span: Span) -> Result<LoopFrame, Diagnostic> {
        self.loops
            .iter()
            .rev()
            .find(|frame| frame.kind == kind)
            .copied()
            .ok_or_else(|| unsupported("a loop control target outside its loop", span))
    }
    fn local(
        &mut self,
        name: &str,
        ty: TypeName,
        parameter_index: Option<usize>,
        storage: h::LocalStorage,
        span: Span,
    ) -> Result<h::LocalId, Diagnostic> {
        hir_value_type(self.program, &ty, span)?;
        let id = h::LocalId(self.locals.len());
        let name_key = key(name);
        let scope = self.current_scope();
        if self.scopes[scope.0]
            .locals
            .iter()
            .any(|local| key(&self.locals[local.0].name) == name_key)
        {
            return Err(unsupported("duplicate locals in one scope", span));
        }
        self.names.entry(name_key.clone()).or_default().push(id);
        let previous = self.symbols.insert(
            name_key.clone(),
            VarType::Scalar(Visibility::Public, ty.clone()),
        );
        self.shadowed_symbols[scope.0].push((name_key, previous));
        self.scopes[scope.0].locals.push(id);
        self.locals.push(h::Local {
            id,
            name: name.into(),
            properties: crate::frontend::semantics::type_properties::properties(self.program, &ty),
            ty,
            parameter_index,
            storage,
            initial_state: if parameter_index.is_some() {
                h::InitialState::Initialized
            } else {
                h::InitialState::Uninitialized
            },
            scope,
            span,
        });
        Ok(id)
    }
    fn lookup(&self, name: &str, span: Span) -> Result<h::LocalId, Diagnostic> {
        self.names
            .get(&key(name))
            .and_then(|stack| stack.last().copied())
            .ok_or_else(|| unsupported(&format!("non-local place '{name}'"), span))
    }
    fn place(&self, id: h::LocalId, span: Span) -> h::Expression {
        h::Expression {
            kind: h::ExpressionKind::Place(h::Place::local(id)),
            ty: self.locals[id.0].ty.clone(),
            category: h::ValueCategory::Place,
            span,
        }
    }
    fn indexed_place(
        &self,
        base: h::Expression,
        indices: &[Expr],
        span: Span,
    ) -> Result<h::Expression, Diagnostic> {
        if indices.len() != 1 {
            return Err(unsupported("multidimensional indexed places", span));
        }
        let TypeName::Array(element) = &base.ty else {
            return Err(unsupported("indexing a non-array place", span));
        };
        let index = self.expression(&indices[0])?;
        if !index.ty.is_integral() {
            return Err(unsupported("non-integral array index", indices[0].span));
        }
        let h::ExpressionKind::Place(mut place) = base.kind else {
            return Err(unsupported("indexing a non-addressable value", span));
        };
        let element_type = *element.clone();
        place
            .projections
            .push(h::Projection::Index(h::IndexProjection {
                index: Box::new(index),
                element_type: element_type.clone(),
            }));
        Ok(h::Expression {
            kind: h::ExpressionKind::Place(place),
            ty: element_type,
            category: h::ValueCategory::Place,
            span,
        })
    }
    fn field_place(
        &self,
        base: h::Expression,
        name: &str,
        span: Span,
    ) -> Result<h::Expression, Diagnostic> {
        if let TypeName::Tuple(elements) = &base.ty {
            let (index, element) = elements
                .iter()
                .enumerate()
                .find(|(index, element)| {
                    element
                        .name
                        .as_ref()
                        .is_some_and(|member| member.eq_ignore_ascii_case(name))
                        || crate::frontend::type_model::TupleElement::positional_name(*index)
                            .eq_ignore_ascii_case(name)
                })
                .ok_or_else(|| unsupported("unknown tuple member", span))?;
            let h::ExpressionKind::Place(mut place) = base.kind else {
                return Err(unsupported("tuple field of a non-addressable value", span));
            };
            place.projections.push(h::Projection::TupleField(index));
            return Ok(h::Expression {
                kind: h::ExpressionKind::Place(place),
                ty: element.ty.clone(),
                category: h::ValueCategory::Place,
                span,
            });
        }
        let field = self
            .fields
            .iter()
            .find(|field| field.owner.same_type(&base.ty) && field.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| unsupported("unresolved structure field place", span))?;
        let h::ExpressionKind::Place(mut place) = base.kind else {
            return Err(unsupported("field of a non-addressable value", span));
        };
        place.projections.push(h::Projection::Field(field.id));
        Ok(h::Expression {
            kind: h::ExpressionKind::Place(place),
            ty: field.ty.clone(),
            category: h::ValueCategory::Place,
            span,
        })
    }
    fn source_place(&self, expr: &Expr) -> Result<h::Expression, Diagnostic> {
        match &expr.kind {
            ExprKind::Variable(name) => Ok(self.place(self.lookup(name, expr.span)?, expr.span)),
            ExprKind::Index { target, args } => {
                self.indexed_place(self.source_place(target)?, args, expr.span)
            }
            ExprKind::MemberAccess {
                object,
                field,
                conditional: false,
            } => self.field_place(self.source_place(object)?, field, expr.span),
            ExprKind::Call {
                name,
                type_args,
                args,
            } if type_args.is_empty()
                && self
                    .lookup(name, expr.span)
                    .ok()
                    .is_some_and(|id| matches!(self.locals[id.0].ty, TypeName::Array(_))) =>
            {
                self.indexed_place(
                    self.place(self.lookup(name, expr.span)?, expr.span),
                    args,
                    expr.span,
                )
            }
            _ => Err(unsupported("this addressable place", expr.span)),
        }
    }
    fn expression(&self, expr: &Expr) -> Result<h::Expression, Diagnostic> {
        let (kind, ty) = match &expr.kind {
            ExprKind::Integer(value) => (
                h::ExpressionKind::Constant(h::Constant::Integer(*value)),
                TypeName::integer_literal(*value),
            ),
            ExprKind::Long(value) => (
                h::ExpressionKind::Constant(h::Constant::Integer(i64::from(*value))),
                TypeName::Int32,
            ),
            ExprKind::LongLong(value) => (
                h::ExpressionKind::Constant(h::Constant::Integer(*value)),
                TypeName::Int64,
            ),
            ExprKind::Single(value) => (
                h::ExpressionKind::Constant(h::Constant::Single(*value)),
                TypeName::Single,
            ),
            ExprKind::Double(value) => (
                h::ExpressionKind::Constant(h::Constant::Double(*value)),
                TypeName::Double,
            ),
            ExprKind::Boolean(value) => (
                h::ExpressionKind::Constant(h::Constant::Boolean(*value)),
                TypeName::Boolean,
            ),
            ExprKind::String(value) => (
                h::ExpressionKind::Constant(h::Constant::String(value.clone())),
                TypeName::String,
            ),
            ExprKind::Nothing => {
                return Err(unsupported("Nothing values in native HIR", expr.span));
            }
            ExprKind::New {
                class_name,
                args,
                initializer,
                member_initializer,
            } => {
                if !args.is_empty() || initializer.is_some() || member_initializer.is_some() {
                    return Err(unsupported(
                        "native Class constructor arguments or initializers",
                        expr.span,
                    ));
                }
                if is_collection(class_name) {
                    (h::ExpressionKind::NewCollection, class_name.clone())
                } else {
                    validate_native_class_creation(self.program, class_name, expr.span)?;
                    (
                        h::ExpressionKind::NewClass(class_name.clone()),
                        class_name.clone(),
                    )
                }
            }
            ExprKind::Convert {
                expr: source,
                target,
                kind,
            } => {
                if *kind != crate::ConversionKind::Convert {
                    return Err(unsupported(
                        "native DirectCast/TryCast reference semantics",
                        expr.span,
                    ));
                }
                let value = self.expression(source)?;
                return convert(value, target, h::Conversion::NumericChecked);
            }
            ExprKind::Interpolated(parts) => return self.interpolation(parts, expr.span),
            ExprKind::TupleLiteral(elements) => {
                let values = elements
                    .iter()
                    .map(|element| self.expression(&element.value))
                    .collect::<Result<Vec<_>, _>>()?;
                let ty = TypeName::Tuple(
                    elements
                        .iter()
                        .zip(&values)
                        .map(
                            |(element, value)| crate::frontend::type_model::TupleElement {
                                name: element.name.clone(),
                                ty: value.ty.clone(),
                            },
                        )
                        .collect(),
                );
                (h::ExpressionKind::Tuple(values), ty)
            }
            ExprKind::Variable(name) => {
                let id = self.lookup(name, expr.span)?;
                let place = self.place(id, expr.span);
                let ty = place.ty.clone();
                (h::ExpressionKind::Load(Box::new(place)), ty)
            }
            ExprKind::Unary {
                op: crate::UnaryOp::LogicalNot,
                expr: operand,
            } => {
                let value = boolean_condition(self.expression(operand)?)?;
                let false_value = h::Expression {
                    kind: h::ExpressionKind::Constant(h::Constant::Boolean(false)),
                    ty: TypeName::Boolean,
                    category: h::ValueCategory::Value,
                    span: expr.span,
                };
                (
                    h::ExpressionKind::Compare {
                        operation: h::ComparisonOp::Equal,
                        left: Box::new(value),
                        right: Box::new(false_value),
                    },
                    TypeName::Boolean,
                )
            }
            ExprKind::Index { .. } => {
                let place = self.source_place(expr)?;
                let ty = place.ty.clone();
                (h::ExpressionKind::Load(Box::new(place)), ty)
            }
            ExprKind::MemberAccess {
                object,
                field,
                conditional: false,
            } if field.eq_ignore_ascii_case("Count")
                && matches!(&object.kind, ExprKind::Variable(name) if self.lookup(name, object.span).is_ok_and(|id| is_collection(&self.locals[id.0].ty))) =>
            {
                let collection = self.expression(object)?;
                (
                    h::ExpressionKind::CollectionCount(Box::new(collection)),
                    TypeName::Int32,
                )
            }
            ExprKind::MemberAccess {
                conditional: false, ..
            } => {
                let place = self.source_place(expr)?;
                let ty = place.ty.clone();
                (h::ExpressionKind::Load(Box::new(place)), ty)
            }
            ExprKind::Binary { left, op, right } => {
                if matches!(op, crate::BinaryOp::Is | crate::BinaryOp::IsNot) {
                    let left = if matches!(left.kind, ExprKind::Nothing) {
                        let right_value = self.expression(right)?;
                        let left_value = null_reference(self.program, &right_value.ty, left.span)?;
                        return Ok(h::Expression {
                            kind: h::ExpressionKind::ReferenceIdentity {
                                left: Box::new(left_value),
                                right: Box::new(right_value),
                                negated: *op == crate::BinaryOp::IsNot,
                            },
                            ty: TypeName::Boolean,
                            category: h::ValueCategory::Value,
                            span: expr.span,
                        });
                    } else {
                        self.expression(left)?
                    };
                    let right = if matches!(right.kind, ExprKind::Nothing) {
                        null_reference(self.program, &left.ty, right.span)?
                    } else {
                        self.expression(right)?
                    };
                    if !left.ty.same_type(&right.ty)
                        || !matches!(&left.ty, TypeName::User(name) if self.program.classes.iter().any(|class| class.name.eq_ignore_ascii_case(name)))
                    {
                        return Err(unsupported(
                            "identity between these reference types",
                            expr.span,
                        ));
                    }
                    return Ok(h::Expression {
                        kind: h::ExpressionKind::ReferenceIdentity {
                            left: Box::new(left),
                            right: Box::new(right),
                            negated: *op == crate::BinaryOp::IsNot,
                        },
                        ty: TypeName::Boolean,
                        category: h::ValueCategory::Value,
                        span: expr.span,
                    });
                }
                if matches!(op, crate::BinaryOp::Concat) {
                    let left = self.expression(left)?;
                    let right = self.expression(right)?;
                    let left = self.stringify(left)?;
                    let right = self.stringify(right)?;
                    return Ok(h::Expression {
                        kind: h::ExpressionKind::StringConcat {
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        ty: TypeName::String,
                        category: h::ValueCategory::Value,
                        span: expr.span,
                    });
                }
                if let Some(operation) = h::ComparisonOp::from_ast(*op) {
                    let left = self.expression(left)?;
                    let right = self.expression(right)?;
                    if left.ty == TypeName::String && right.ty == TypeName::String {
                        return Ok(h::Expression {
                            kind: h::ExpressionKind::StringCompare {
                                operation,
                                left: Box::new(left),
                                right: Box::new(right),
                                text: self.option_compare == crate::OptionCompare::Text,
                            },
                            ty: TypeName::Boolean,
                            category: h::ValueCategory::Value,
                            span: expr.span,
                        });
                    }
                    let operand_type = if left.ty.same_type(&TypeName::Boolean)
                        && right.ty.same_type(&TypeName::Boolean)
                        && matches!(
                            operation,
                            h::ComparisonOp::Equal | h::ComparisonOp::NotEqual
                        ) {
                        TypeName::Boolean
                    } else {
                        arithmetic::signature(ArithmeticOp::Add, &left.ty, &right.ty)
                            .ok_or_else(|| {
                                unsupported("comparison of these operand types", expr.span)
                            })?
                            .operand_type
                    };
                    let left = convert(left, &operand_type, h::Conversion::NumericChecked)?;
                    let right = convert(right, &operand_type, h::Conversion::NumericChecked)?;
                    return Ok(h::Expression {
                        kind: h::ExpressionKind::Compare {
                            operation,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        ty: TypeName::Boolean,
                        category: h::ValueCategory::Value,
                        span: expr.span,
                    });
                }
                let operation = ArithmeticOp::from_ast(*op)
                    .ok_or_else(|| unsupported("this binary operator", expr.span))?;
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                let signature =
                    arithmetic::signature(operation, &left.ty, &right.ty).ok_or_else(|| {
                        unsupported("arithmetic without a common primitive type", expr.span)
                    })?;
                let conversion = if operation == ArithmeticOp::IntegerDivide {
                    h::Conversion::TruncateToInteger
                } else {
                    h::Conversion::NumericChecked
                };
                let left = convert(left, &signature.operand_type, conversion)?;
                let right = convert(right, &signature.operand_type, conversion)?;
                let ty = signature.result_type.clone();
                (
                    h::ExpressionKind::Arithmetic {
                        operation,
                        signature,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    ty,
                )
            }
            ExprKind::MemberCall {
                object,
                method,
                type_args,
                args,
                conditional: false,
            } if method.eq_ignore_ascii_case("Item")
                && type_args.is_empty()
                && matches!(&object.kind, ExprKind::Variable(name) if self.lookup(name, object.span).is_ok_and(|id| is_collection(&self.locals[id.0].ty))) =>
            {
                if args.len() != 1 {
                    return Err(unsupported("Collection.Item argument count", expr.span));
                }
                let collection = self.expression(object)?;
                let index = convert(
                    self.expression(&args[0])?,
                    &TypeName::Int64,
                    h::Conversion::NumericChecked,
                )?;
                (
                    h::ExpressionKind::CollectionItem {
                        collection: Box::new(collection),
                        index: Box::new(index),
                    },
                    TypeName::Variant,
                )
            }
            ExprKind::MemberCall {
                object,
                method,
                type_args,
                args,
                conditional: false,
            } if matches!(&object.kind, ExprKind::Variable(_)) => {
                let ExprKind::Variable(owner) = &object.kind else {
                    unreachable!()
                };
                if self.lookup(owner, object.span).is_ok() {
                    return Err(unsupported("native member calls on values", expr.span));
                }
                let qualified = Expr {
                    kind: ExprKind::Call {
                        name: format!("{owner}.{method}"),
                        type_args: type_args.clone(),
                        args: args.clone(),
                    },
                    span: expr.span,
                };
                return self.expression(&qualified);
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                if name.eq_ignore_ascii_case("Len")
                    && type_args.is_empty()
                    && args.len() == 1
                    && !self.signatures.functions.contains_key(&key(name))
                    && !self
                        .signatures
                        .functions
                        .contains_key(&key(&format!("{}.{}", self.owner, name)))
                {
                    let value = self.expression(&args[0])?;
                    if value.ty == TypeName::String {
                        return Ok(h::Expression {
                            kind: h::ExpressionKind::StringLen(Box::new(value)),
                            ty: TypeName::Int32,
                            category: h::ValueCategory::Value,
                            span: expr.span,
                        });
                    }
                }
                if type_args.is_empty()
                    && let Ok(id) = self.lookup(name, expr.span)
                    && matches!(self.locals[id.0].ty, TypeName::Array(_))
                {
                    let place = self.indexed_place(self.place(id, expr.span), args, expr.span)?;
                    let ty = place.ty.clone();
                    return Ok(h::Expression {
                        kind: h::ExpressionKind::Load(Box::new(place)),
                        ty,
                        category: h::ValueCategory::Value,
                        span: expr.span,
                    });
                }
                if matches!(self.symbols.get(&key(name)), Some(VarType::Array(..))) {
                    return Err(unsupported(
                        "native module-level array storage and indexing",
                        expr.span,
                    ));
                }
                if !type_args.is_empty() {
                    return Err(unsupported("generic calls", expr.span));
                }
                let resolved_name = if self.signatures.functions.contains_key(&key(name)) {
                    name.clone()
                } else {
                    format!("{}.{}", self.owner, name)
                };
                let mut candidates = self
                    .signatures
                    .functions
                    .get(&key(&resolved_name))
                    .cloned()
                    .unwrap_or_default();
                if let Some((owner, simple)) = name.rsplit_once('.') {
                    candidates.extend(
                        self.signatures
                            .functions
                            .get(&key(simple))
                            .into_iter()
                            .flatten()
                            .filter(|candidate| {
                                self.program.functions.iter().any(|function| {
                                    key(&function.name) == key(simple)
                                        && self.program.owners.get(&function.span).is_some_and(
                                            |actual| actual.eq_ignore_ascii_case(owner),
                                        )
                                        && function.params.len() == candidate.params.len()
                                        && function
                                            .params
                                            .iter()
                                            .zip(&candidate.params)
                                            .all(|(a, b)| a.mode == b.mode && a.ty.same_type(&b.ty))
                                })
                            })
                            .cloned(),
                    );
                }
                if candidates.is_empty() {
                    return Err(unsupported("builtin or external calls", expr.span));
                }
                let context = Context::Sub { is_async: false };
                let selected = resolve_overload(
                    "Function",
                    name,
                    &candidates,
                    args,
                    expr.span,
                    ExprValidation::new(
                        &self.symbols,
                        self.types,
                        self.signatures,
                        &context,
                        self.options,
                    ),
                )?;
                let (index, function) =
                    self.program
                        .functions
                        .iter()
                        .enumerate()
                        .find(|(_, function)| {
                            (key(&function.name) == key(&resolved_name)
                                || name.rsplit_once('.').is_some_and(|(owner, simple)| {
                                    key(&function.name) == key(simple)
                                        && self.program.owners.get(&function.span).is_some_and(
                                            |actual| actual.eq_ignore_ascii_case(owner),
                                        )
                                }))
                                && function.params.len() == selected.params.len()
                                && function
                                    .params
                                    .iter()
                                    .zip(&selected.params)
                                    .all(|(a, b)| a.ty.same_type(&b.ty) && a.mode == b.mode)
                                && function.type_params == selected.type_params
                        })
                        .ok_or_else(|| {
                            unsupported("external or specialized call targets", expr.span)
                        })?;
                check_function(self.program, function)?;
                if args.len() != function.params.len() {
                    return Err(unsupported("omitted arguments", expr.span));
                }
                let mut arguments = Vec::new();
                for (arg, param) in args.iter().zip(&function.params) {
                    let value = if param.mode != PassingMode::ByVal {
                        let place = self.source_place(arg)?;
                        if !place.ty.same_type(&param.ty) {
                            return Err(unsupported(
                                "ByRef conversions requiring temporaries",
                                arg.span,
                            ));
                        }
                        h::Expression {
                            ty: place.ty.clone(),
                            span: arg.span,
                            category: if param.mode == PassingMode::ByRefReadOnly {
                                h::ValueCategory::ImmutableReference
                            } else {
                                h::ValueCategory::MutableReference
                            },
                            kind: if param.mode == PassingMode::ByRefReadOnly {
                                h::ExpressionKind::BorrowImmutable(Box::new(place))
                            } else {
                                h::ExpressionKind::BorrowMutable(Box::new(place))
                            },
                        }
                    } else {
                        convert(
                            self.expression(arg)?,
                            &param.ty,
                            h::Conversion::NumericChecked,
                        )?
                    };
                    arguments.push(h::CallArgument {
                        mode: match param.mode {
                            PassingMode::ByVal => h::ArgumentMode::ByVal,
                            PassingMode::ByRef => h::ArgumentMode::BorrowMutable,
                            PassingMode::ByRefReadOnly => h::ArgumentMode::BorrowImmutable,
                        },
                        value,
                    });
                }
                (
                    h::ExpressionKind::Call {
                        function: h::BodyFunctionId(index),
                        signature: h::CallSignature {
                            parameter_types: function
                                .params
                                .iter()
                                .map(|param| param.ty.clone())
                                .collect(),
                            parameter_modes: arguments.iter().map(|arg| arg.mode).collect(),
                            return_type: function.return_type.clone(),
                        },
                        arguments,
                    },
                    function.return_type.clone(),
                )
            }
            _ => return Err(unsupported("this expression", expr.span)),
        };
        hir_value_type(self.program, &ty, expr.span)?;
        Ok(h::Expression {
            kind,
            ty,
            category: h::ValueCategory::Value,
            span: expr.span,
        })
    }

    fn interpolation(
        &self,
        parts: &[crate::InterpolationPart],
        span: Span,
    ) -> Result<h::Expression, Diagnostic> {
        let literal = |text: String| h::Expression {
            kind: h::ExpressionKind::Constant(h::Constant::String(text)),
            ty: TypeName::String,
            category: h::ValueCategory::Value,
            span,
        };
        let mut result = literal(String::new());
        for part in parts {
            let next = match part {
                crate::InterpolationPart::Literal(text) => literal(text.clone()),
                crate::InterpolationPart::Value {
                    expr,
                    alignment,
                    format,
                } => {
                    if alignment.is_some() {
                        return Err(unsupported(
                            "interpolation alignment in native String",
                            expr.span,
                        ));
                    }
                    let value = self.expression(expr)?;
                    if value.ty == TypeName::String && format.as_ref().is_none_or(String::is_empty)
                    {
                        value
                    } else {
                        let decimals = match format.as_deref().filter(|text| !text.is_empty()) {
                            None => None,
                            Some(text)
                                if text.starts_with("0.")
                                    && text[2..].bytes().all(|byte| byte == b'0')
                                    && (1..=6).contains(&(text.len() - 2)) =>
                            {
                                Some((text.len() - 2) as u8)
                            }
                            _ => {
                                return Err(unsupported(
                                    "this interpolation format in native String",
                                    expr.span,
                                ));
                            }
                        };
                        if !(value.ty.is_integral()
                            || matches!(
                                value.ty,
                                TypeName::Single | TypeName::Double | TypeName::Boolean
                            ))
                        {
                            return Err(unsupported(
                                "interpolation of this type in native String",
                                expr.span,
                            ));
                        }
                        h::Expression {
                            kind: h::ExpressionKind::StringFormat {
                                value: Box::new(value),
                                decimals,
                            },
                            ty: TypeName::String,
                            category: h::ValueCategory::Value,
                            span: expr.span,
                        }
                    }
                }
            };
            result = h::Expression {
                kind: h::ExpressionKind::StringConcat {
                    left: Box::new(result),
                    right: Box::new(next),
                },
                ty: TypeName::String,
                category: h::ValueCategory::Value,
                span,
            };
        }
        Ok(result)
    }

    fn stringify(&self, value: h::Expression) -> Result<h::Expression, Diagnostic> {
        if value.ty == TypeName::String {
            return Ok(value);
        }
        if value.ty.is_integral()
            || matches!(
                value.ty,
                TypeName::Single | TypeName::Double | TypeName::Boolean
            )
        {
            let span = value.span;
            Ok(h::Expression {
                kind: h::ExpressionKind::StringFormat {
                    value: Box::new(value),
                    decimals: None,
                },
                ty: TypeName::String,
                category: h::ValueCategory::Value,
                span,
            })
        } else {
            Err(unsupported("String conversion of this type", value.span))
        }
    }
}
