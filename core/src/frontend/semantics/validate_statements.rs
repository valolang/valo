use super::*;
use crate::UsingResource;
use std::collections::HashSet;

fn validate_const_decl(
    name: &str,
    ty: &Option<TypeName>,
    value: &Expr,
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<(String, VarType), Diagnostic> {
    ensure_const_expr(value, validation.symbols, validation.types)?;
    let value_type = validate_expr(
        value,
        validation.symbols,
        validation.types,
        validation.signatures,
        validation.context,
        validation.option_explicit,
    )?;
    let const_type = ty.clone().unwrap_or(value_type.clone());
    ensure_known_type(&const_type, validation.types, span)?;
    ensure_assignable_expr(&const_type, &value_type, value, validation.types, span)?;
    let key = key(name);
    if validation.symbols.contains_key(&key) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
            format!("Variable '{}' is already declared", name),
            Some(span),
        ));
    }
    Ok((key, VarType::Const(Visibility::Public, const_type)))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LoopContext {
    pub(super) for_depth: usize,
    pub(super) while_depth: usize,
    pub(super) do_depth: usize,
}

impl LoopContext {
    pub(super) fn in_for(mut self) -> Self {
        self.for_depth += 1;
        self
    }

    pub(super) fn in_while(mut self) -> Self {
        self.while_depth += 1;
        self
    }

    pub(super) fn in_do(mut self) -> Self {
        self.do_depth += 1;
        self
    }
}

pub struct StmtValidation<'a, 'ctx> {
    pub(super) symbols: &'a mut HashMap<String, VarType>,
    pub(super) types: &'a TypeRegistry,
    pub(super) signatures: &'a Signatures,
    pub(super) context: &'a mut Context<'ctx>,
    pub(super) loop_context: LoopContext,
    pub(super) in_with: bool,
    pub(super) option_explicit: bool,
}

pub fn validate_statements(
    statements: &[Stmt],
    validation: &mut StmtValidation<'_, '_>,
) -> Result<(), Diagnostic> {
    let symbols = &mut *validation.symbols;
    let types = validation.types;
    let signatures = validation.signatures;
    let mut context = &mut *validation.context;
    let loop_context = validation.loop_context;
    let in_with = validation.in_with;
    let option_explicit = validation.option_explicit;
    validate_labels(statements, context)?;
    for stmt in statements {
        if !in_with && stmt_uses_with_target(stmt, context) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Dot member access requires an active With block",
                Some(stmt_span(stmt, context)),
            ));
        }
        match stmt {
            Stmt::Dim {
                name,
                ty,
                array,
                as_new,
                new_args,
                initializer,
                span,
                ..
            }
            | Stmt::Static {
                name,
                ty,
                array,
                as_new,
                new_args,
                initializer,
                span,
                ..
            } => {
                let ty = declared_variable_type(
                    ty,
                    initializer,
                    *span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                ensure_known_type(&ty, types, *span)?;
                validate_as_new(
                    *as_new,
                    &ty,
                    new_args,
                    *span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                if let Some(initializer) = initializer {
                    if array.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Array declarations cannot use an initializer",
                            Some(initializer.span),
                        ));
                    }
                    let source_type = validate_expr(
                        initializer,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?;
                    ensure_assignable_expr(
                        &ty,
                        &source_type,
                        initializer,
                        types,
                        initializer.span,
                    )?;
                }
                let key = key(name);
                if symbols.contains_key(&key) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                        format!("Variable '{}' is already declared", name),
                        Some(*span),
                    ));
                }
                let var_type = if let Some(array_decl) = array {
                    VarType::Array(
                        Visibility::Public,
                        ty,
                        matches!(array_decl, ArrayDecl::Dynamic),
                    )
                } else {
                    VarType::Scalar(Visibility::Public, ty)
                };
                symbols.insert(key, var_type);
            }
            Stmt::DimMany { decls, .. } | Stmt::StaticMany { decls, .. } => {
                let mut seen = HashSet::new();
                for decl in decls {
                    let decl_key = key(&decl.name);
                    if !seen.insert(decl_key.clone()) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                            format!("Variable '{}' is declared more than once", decl.name),
                            Some(decl.span),
                        ));
                    }
                    let ty = declared_variable_type(
                        &decl.ty,
                        &decl.initializer,
                        decl.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    ensure_known_type(&ty, types, decl.span)?;
                    validate_as_new(
                        decl.as_new,
                        &ty,
                        &decl.new_args,
                        decl.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    if let Some(initializer) = &decl.initializer {
                        if decl.array.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                "Array declarations cannot use an initializer",
                                Some(initializer.span),
                            ));
                        }
                        let source_type = validate_expr(
                            initializer,
                            symbols,
                            types,
                            signatures,
                            context,
                            option_explicit,
                        )?;
                        ensure_assignable_expr(
                            &ty,
                            &source_type,
                            initializer,
                            types,
                            initializer.span,
                        )?;
                    }
                    if symbols.contains_key(&decl_key) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                            format!("Variable '{}' is already declared", decl.name),
                            Some(decl.span),
                        ));
                    }
                    let var_type = if let Some(array_decl) = &decl.array {
                        VarType::Array(
                            Visibility::Public,
                            ty,
                            matches!(array_decl, ArrayDecl::Dynamic),
                        )
                    } else {
                        VarType::Scalar(Visibility::Public, ty)
                    };
                    symbols.insert(decl_key, var_type);
                }
            }
            Stmt::Const {
                name,
                ty,
                value,
                span,
            } => {
                let (const_key, const_type) = validate_const_decl(
                    name,
                    ty,
                    value,
                    *span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                symbols.insert(const_key, const_type);
            }
            Stmt::ConstMany { consts, .. } => {
                for const_decl in consts {
                    let (const_key, const_type) = validate_const_decl(
                        &const_decl.name,
                        &const_decl.ty,
                        &const_decl.value,
                        const_decl.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    symbols.insert(const_key, const_type);
                }
            }
            Stmt::Assign { target, expr, span } => {
                let expr_type = class_field_expr_type(expr, symbols, types, context)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        validate_expr(expr, symbols, types, signatures, context, option_explicit)
                    })?;
                let target_type = validate_assignment_target(
                    target,
                    &expr_type,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                ensure_assignable_expr(&target_type, &expr_type, expr, types, *span)?;
            }
            Stmt::SetAssign { target, expr, span } => {
                let expr_type = class_field_expr_type(expr, symbols, types, context)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        validate_expr(expr, symbols, types, signatures, context, option_explicit)
                    })?;
                let target_type = validate_assignment_target(
                    target,
                    &expr_type,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                if !target_type.same_type(&TypeName::Variant) {
                    ensure_class_type(
                        &target_type,
                        types,
                        *span,
                        "Set target must be a class type",
                    )?;
                }
                ensure_assignable_expr(&target_type, &expr_type, expr, types, *span)?;
            }
            Stmt::ConsoleCall { method, args, .. } => {
                if !matches!(
                    method.to_ascii_lowercase().as_str(),
                    "writeline" | "write" | "readline"
                ) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Unknown Console method: {}", method),
                        Some(stmt_span(stmt, context)),
                    ));
                }
                for arg in args {
                    validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
                }
            }
            Stmt::DebugPrint { args, .. } => {
                for arg in args {
                    validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
                }
            }
            Stmt::OpenFile {
                path,
                number,
                record_len,
                ..
            } => {
                validate_expr(path, symbols, types, signatures, context, option_explicit)?;
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                if let Some(record_len) = record_len {
                    validate_expr(
                        record_len,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?;
                }
            }
            Stmt::CloseFile { numbers, .. } => {
                for number in numbers {
                    validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                }
            }
            Stmt::LineInput { number, target, .. } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                validate_assignment_target(
                    target,
                    &TypeName::String,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
            }
            Stmt::InputFile {
                number, targets, ..
            } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                for target in targets {
                    validate_assignment_target(
                        target,
                        &TypeName::Variant,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?;
                }
            }
            Stmt::PrintFile { number, items, .. } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                for item in items {
                    validate_expr(
                        &item.expr,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?;
                }
            }
            Stmt::WriteFile { number, args, .. } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                for arg in args {
                    validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
                }
            }
            Stmt::GetFile {
                number,
                position,
                target,
                ..
            } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                if let Some(position) = position {
                    validate_expr(
                        position,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?;
                }
                validate_assignment_target(
                    target,
                    &TypeName::Variant,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
            }
            Stmt::PutFile {
                number,
                position,
                expr,
                ..
            } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                if let Some(position) = position {
                    validate_expr(
                        position,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?;
                }
                validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
            }
            Stmt::SeekFile {
                number, position, ..
            } => {
                validate_expr(number, symbols, types, signatures, context, option_explicit)?;
                validate_expr(
                    position,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
            }
            Stmt::NameFile {
                old_path, new_path, ..
            } => {
                validate_expr(
                    old_path,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                validate_expr(
                    new_path,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
            }
            Stmt::End { .. } => {}
            Stmt::SubCall { name, args, span } => {
                validate_sub_call(
                    name,
                    args,
                    *span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
            }
            Stmt::MemberSubCall {
                object,
                method,
                args,
                span,
            } => {
                if let ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("VBA")
                {
                    validate_sub_call(
                        method,
                        args,
                        *span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    continue;
                }
                if let ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Err")
                {
                    if method.eq_ignore_ascii_case("Clear") && args.is_empty() {
                        continue;
                    }
                    if method.eq_ignore_ascii_case("Raise") {
                        validate_expr(
                            &Expr {
                                kind: ExprKind::MemberCall {
                                    object: Box::new(object.clone()),
                                    method: method.clone(),
                                    type_args: Vec::new(),
                                    args: args.clone(),
                                },
                                span: *span,
                            },
                            symbols,
                            types,
                            signatures,
                            context,
                            option_explicit,
                        )?;
                        continue;
                    }
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Err only supports Clear() and Raise()",
                        Some(*span),
                    ));
                }
                let object_type = class_field_object_type(object, symbols, types, context)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        validate_expr(object, symbols, types, signatures, context, option_explicit)
                    })?;
                validate_method_call(
                    &object_type,
                    method,
                    args,
                    false,
                    *span,
                    symbols,
                    types,
                    signatures,
                    context.current_class(),
                    context,
                    option_explicit,
                )?;
            }
            Stmt::RaiseEvent { name, args, span } => {
                let Some(class_name) = context.current_class() else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "RaiseEvent is only valid inside the declaring class",
                        Some(*span),
                    ));
                };
                let class_sig = types
                    .get_class(class_name)
                    .expect("current class validated");
                let Some(event_sig) = class_sig.events.get(&key(name)) else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Class '{}' has no event '{}'", class_sig.name, name),
                        Some(*span),
                    ));
                };
                validate_arguments(
                    "Event",
                    event_sig,
                    args,
                    *span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
            }
            Stmt::AddHandler {
                event,
                handler,
                span,
            }
            | Stmt::RemoveHandler {
                event,
                handler,
                span,
            } => {
                let _event_ty =
                    validate_expr(event, symbols, types, signatures, context, option_explicit)?;
                let _handler_ty = validate_expr(
                    handler,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;

                if !matches!(event.kind, ExprKind::MemberAccess { .. }) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Event argument must be a member access (e.g. obj.EventName)",
                        Some(event.span),
                    ));
                }
                let _ = span;
            }
            Stmt::Await { expr, .. } => {
                if !context.allows_await() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::CONTROL_FLOW,
                        "Await is only allowed inside Async Sub or Async Function",
                        Some(expr.span),
                    ));
                }
                validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
            }
            Stmt::Return { expr, span } => {
                let expr_type =
                    validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
                match &mut context {
                    Context::Sub { .. }
                    | Context::MethodSub { .. }
                    | Context::PropertyLetSet { .. } => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Return is only allowed inside Function or Property Get",
                            Some(*span),
                        ));
                    }
                    Context::Function {
                        return_type,
                        is_iterator,
                        saw_return,
                        ..
                    }
                    | Context::MethodFunction {
                        return_type,
                        is_iterator,
                        saw_return,
                        ..
                    }
                    | Context::PropertyGet {
                        return_type,
                        is_iterator,
                        saw_return,
                        ..
                    } => {
                        if *is_iterator {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                                "Return is not allowed inside Iterator; use Yield or Exit Function",
                                Some(*span),
                            ));
                        }
                        ensure_assignable_expr(return_type, &expr_type, expr, types, *span)?;
                        **saw_return = true;
                    }
                }
            }
            Stmt::Yield { expr, span } => {
                let expr_type =
                    validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
                match &mut context {
                    Context::Function {
                        return_type,
                        is_iterator,
                        saw_yield,
                        ..
                    }
                    | Context::MethodFunction {
                        return_type,
                        is_iterator,
                        saw_yield,
                        ..
                    }
                    | Context::PropertyGet {
                        return_type,
                        is_iterator,
                        saw_yield,
                        ..
                    } if *is_iterator => {
                        ensure_assignable_expr(return_type, &expr_type, expr, types, *span)?;
                        **saw_yield = true;
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Yield is only allowed inside Iterator functions",
                            Some(*span),
                        ));
                    }
                }
            }
            Stmt::Throw { expr, .. } => {
                validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
            }
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                ..
            } => {
                ensure_assignable(
                    &TypeName::Boolean,
                    &validate_expr(
                        condition,
                        symbols,
                        types,
                        signatures,
                        context,
                        option_explicit,
                    )?,
                    condition.span,
                )?;
                validate_statements(
                    then_body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context,
                        in_with,
                        option_explicit,
                    },
                )?;
                for branch in elseif_branches {
                    ensure_assignable(
                        &TypeName::Boolean,
                        &validate_expr(
                            &branch.condition,
                            symbols,
                            types,
                            signatures,
                            context,
                            option_explicit,
                        )?,
                        branch.condition.span,
                    )?;
                    validate_statements(
                        &branch.body,
                        &mut StmtValidation {
                            symbols,
                            types,
                            signatures,
                            context: &mut context.reborrow(),
                            loop_context,
                            in_with,
                            option_explicit,
                        },
                    )?;
                }
                validate_statements(
                    else_body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context,
                        in_with,
                        option_explicit,
                    },
                )?;
            }
            Stmt::SelectCase {
                subject,
                branches,
                else_body,
                ..
            } => {
                let subject_type = validate_expr(
                    subject,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                for branch in branches {
                    for item in &branch.items {
                        validate_case_item(
                            item,
                            &subject_type,
                            symbols,
                            types,
                            signatures,
                            context,
                            option_explicit,
                        )?;
                    }
                    validate_statements(
                        &branch.body,
                        &mut StmtValidation {
                            symbols,
                            types,
                            signatures,
                            context: &mut context.reborrow(),
                            loop_context,
                            in_with,
                            option_explicit,
                        },
                    )?;
                }
                validate_statements(
                    else_body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context,
                        in_with,
                        option_explicit,
                    },
                )?;
            }
            Stmt::While {
                condition, body, ..
            } => {
                validate_expr(
                    condition,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                validate_statements(
                    body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context: loop_context.in_while(),
                        in_with,
                        option_explicit,
                    },
                )?;
            }
            Stmt::DoLoop {
                condition, body, ..
            } => {
                match condition {
                    DoLoopCondition::Infinite => {}
                    DoLoopCondition::PreWhile(condition)
                    | DoLoopCondition::PreUntil(condition)
                    | DoLoopCondition::PostWhile(condition)
                    | DoLoopCondition::PostUntil(condition) => {
                        ensure_assignable(
                            &TypeName::Boolean,
                            &validate_expr(
                                condition,
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            )?,
                            condition.span,
                        )?;
                    }
                }
                validate_statements(
                    body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context: loop_context.in_do(),
                        in_with,
                        option_explicit,
                    },
                )?;
            }
            Stmt::For {
                variable,
                start,
                end,
                step,
                next_variable,
                body,
                span,
            } => {
                let Some(ty) = symbols.get(&key(variable)) else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", variable),
                        Some(*span),
                    ));
                };

                if !matches!(ty.scalar_type(), Some(scalar) if scalar.same_type(&TypeName::Integer))
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!("For loop variable '{}' must be Integer", variable),
                        Some(*span),
                    ));
                }

                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(start, symbols, types, signatures, context, option_explicit)?,
                    start.span,
                )?;
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(end, symbols, types, signatures, context, option_explicit)?,
                    end.span,
                )?;
                if let Some(step) = step {
                    ensure_assignable(
                        &TypeName::Integer,
                        &validate_expr(step, symbols, types, signatures, context, option_explicit)?,
                        step.span,
                    )?;
                }
                if let Some((next_variable, next_span)) = next_variable
                    && !next_variable.eq_ignore_ascii_case(variable)
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!(
                            "Next variable '{}' does not match For variable '{}'",
                            next_variable, variable
                        ),
                        Some(*next_span),
                    ));
                }
                validate_statements(
                    body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context: loop_context.in_for(),
                        in_with,
                        option_explicit,
                    },
                )?;
            }
            Stmt::ForEach {
                variable,
                iterable,
                next_variable,
                body,
                span,
            } => {
                let Some(loop_type) = symbols.get(&key(variable)).cloned() else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", variable),
                        Some(*span),
                    ));
                };
                let Some(loop_type) = loop_type.scalar_type() else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        format!("Array variable '{}' cannot be used as a scalar", variable),
                        Some(*span),
                    ));
                };
                let array_type = validate_for_each_iterable_expr(
                    iterable,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                ensure_assignable(&loop_type, &array_type, *span)?;
                if let Some((next_variable, next_span)) = next_variable
                    && !next_variable.eq_ignore_ascii_case(variable)
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!(
                            "Next variable '{}' does not match For Each variable '{}'",
                            next_variable, variable
                        ),
                        Some(*next_span),
                    ));
                }
                validate_statements(
                    body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context: loop_context.in_for(),
                        in_with,
                        option_explicit,
                    },
                )?;
            }
            Stmt::ReDim {
                target,
                dims,
                preserve,
                span,
                ..
            } => {
                let array_info = get_redim_target_array_info(
                    target,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                match array_info {
                    Some(true) => {} // Dynamic array, OK
                    Some(false) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "ReDim cannot be used on fixed-size arrays",
                            Some(*span),
                        ));
                    }
                    None => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "ReDim target must be a dynamic array or Variant",
                            Some(*span),
                        ));
                    }
                }
                for (lower, upper) in dims {
                    let upper_type = class_field_expr_type(upper, symbols, types, context)
                        .map(Ok)
                        .unwrap_or_else(|| {
                            validate_expr(
                                upper,
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            )
                        })?;
                    ensure_assignable(&TypeName::Integer, &upper_type, upper.span)?;
                    if let Some(lower) = lower {
                        let lower_type = class_field_expr_type(lower, symbols, types, context)
                            .map(Ok)
                            .unwrap_or_else(|| {
                                validate_expr(
                                    lower,
                                    symbols,
                                    types,
                                    signatures,
                                    context,
                                    option_explicit,
                                )
                            })?;
                        ensure_assignable(&TypeName::Integer, &lower_type, lower.span)?;
                    }
                }
                if *preserve && dims.len() > 1 {
                    // We'll handle deeper Preserve checks at runtime or here?
                    // VBA: Only the last dimension can be resized if Preserve is used.
                    // But we might not know the original dimension count yet if it's dynamic.
                }
            }
            Stmt::Erase { target, span } => {
                if get_redim_target_array_info(
                    target,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?
                .is_none()
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "Erase target must be an array or Variant",
                        Some(*span),
                    ));
                }
            }
            Stmt::LSet { target, expr, span } | Stmt::RSet { target, expr, span } => {
                let target_ty = super::validate_expressions::validate_assignment_target(
                    target,
                    &TypeName::String,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                if !target_ty.same_type(&TypeName::String)
                    && !target_ty.same_type(&TypeName::Variant)
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "{} requires a String or Variant variable, not {}",
                            if matches!(stmt, Stmt::LSet { .. }) {
                                "LSet"
                            } else {
                                "RSet"
                            },
                            target_ty.display_name()
                        ),
                        Some(*span),
                    ));
                }
                let expr_ty = super::validate_expressions::validate_expr(
                    expr,
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                ensure_assignable(&TypeName::String, &expr_ty, expr.span)?;
            }
            Stmt::Label { .. } => {}
            Stmt::GoTo { .. } => {}
            Stmt::OnError { .. } => {}
            Stmt::Resume { .. } => {}
            Stmt::With { target, body, .. } => {
                validate_expr(target, symbols, types, signatures, context, option_explicit)?;
                validate_statements(
                    body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context,
                        in_with: true,
                        option_explicit,
                    },
                )?;
            }
            Stmt::Using {
                resource,
                body,
                span,
            } => match resource {
                UsingResource::Declaration(decl) => {
                    if decl.array.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Using resource cannot be an array",
                            Some(decl.span),
                        ));
                    }
                    let ty = declared_variable_type(
                        &decl.ty,
                        &decl.initializer,
                        decl.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    ensure_known_type(&ty, types, decl.span)?;
                    validate_as_new(
                        decl.as_new,
                        &ty,
                        &decl.new_args,
                        decl.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    if let Some(initializer) = &decl.initializer {
                        let source_type = validate_expr(
                            initializer,
                            symbols,
                            types,
                            signatures,
                            context,
                            option_explicit,
                        )?;
                        ensure_assignable_expr(&ty, &source_type, initializer, types, decl.span)?;
                    }
                    validate_using_disposable(&ty, types, decl.span)?;
                    let decl_key = key(&decl.name);
                    if symbols.contains_key(&decl_key) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                            format!("Variable '{}' is already declared", decl.name),
                            Some(decl.span),
                        ));
                    }
                    let mut using_symbols = symbols.clone();
                    using_symbols.insert(decl_key, VarType::Scalar(Visibility::Public, ty));
                    validate_statements(
                        body,
                        &mut StmtValidation {
                            symbols: &mut using_symbols,
                            types,
                            signatures,
                            context: &mut context.reborrow(),
                            loop_context,
                            in_with,
                            option_explicit,
                        },
                    )?;
                }
                UsingResource::Target(expr) => {
                    let ty =
                        validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
                    validate_using_disposable(&ty, types, *span)?;
                    validate_statements(
                        body,
                        &mut StmtValidation {
                            symbols,
                            types,
                            signatures,
                            context: &mut context.reborrow(),
                            loop_context,
                            in_with,
                            option_explicit,
                        },
                    )?;
                }
            },
            Stmt::Exit { target, span } => {
                validate_exit(*target, *span, context, loop_context)?;
            }
            Stmt::TryCatch {
                try_body,
                catch_block,
                finally_body,
                ..
            } => {
                validate_statements(
                    try_body,
                    &mut StmtValidation {
                        symbols,
                        types,
                        signatures,
                        context: &mut context.reborrow(),
                        loop_context,
                        in_with,
                        option_explicit,
                    },
                )?;
                if let Some(catch) = catch_block {
                    let mut catch_symbols = symbols.clone();
                    if let Some(var_name) = &catch.variable {
                        catch_symbols.insert(
                            key(var_name),
                            VarType::Scalar(
                                Visibility::Public,
                                TypeName::User("Error".to_string()),
                            ),
                        );
                    }
                    validate_statements(
                        &catch.body,
                        &mut StmtValidation {
                            symbols: &mut catch_symbols,
                            types,
                            signatures,
                            context: &mut context.reborrow(),
                            loop_context,
                            in_with,
                            option_explicit,
                        },
                    )?;
                }
                if let Some(finally_body) = finally_body {
                    validate_statements(
                        finally_body,
                        &mut StmtValidation {
                            symbols,
                            types,
                            signatures,
                            context: &mut context.reborrow(),
                            loop_context,
                            in_with,
                            option_explicit,
                        },
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn declared_variable_type(
    ty: &Option<TypeName>,
    initializer: &Option<Expr>,
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<TypeName, Diagnostic> {
    if let Some(ty) = ty {
        return Ok(ty.clone());
    }
    if let Some(initializer) = initializer {
        return validate_expr(
            initializer,
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        );
    }
    let _ = span;
    Ok(TypeName::Variant)
}

fn validate_as_new(
    as_new: bool,
    ty: &TypeName,
    args: &[Expr],
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<(), Diagnostic> {
    if !as_new {
        return Ok(());
    }
    let class_name = match ty {
        TypeName::User(_) | TypeName::GenericInstance { .. } => ty.clone(),
        _ => {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "As New requires a class type",
                Some(span),
            ));
        }
    };
    let expr = Expr {
        kind: ExprKind::New {
            class_name,
            args: args.to_vec(),
            initializer: None,
        },
        span,
    };
    validate_expr(
        &expr,
        validation.symbols,
        validation.types,
        validation.signatures,
        validation.context,
        validation.option_explicit,
    )
    .map(|_| ())
}

fn validate_using_disposable(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if ty.same_type(&TypeName::Variant)
        || matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case("Object"))
    {
        return Ok(());
    }

    let TypeName::User(class_name) = ty else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Using target must be a class instance with a parameterless Dispose method",
            Some(span),
        ));
    };

    if types.get(class_name).is_some() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Using target must be a class instance, not a Structure value",
            Some(span),
        ));
    }

    let Some(class_sig) = types.get_class(class_name) else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Class '{}' is not defined", class_name),
            Some(span),
        ));
    };

    let Some(dispose) = class_sig.subs.get("dispose") else {
        if class_sig.functions.contains_key("dispose")
            || class_sig.properties.contains_key("dispose")
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Dispose member used by Using must be a Sub method",
                Some(span),
            ));
        }
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Using target class '{}' has no Dispose method",
                class_sig.name
            ),
            Some(span),
        ));
    };

    if !dispose.params.is_empty() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Dispose method used by Using must be parameterless",
            Some(span),
        ));
    }

    Ok(())
}

fn validate_labels(statements: &[Stmt], context: &Context<'_>) -> Result<(), Diagnostic> {
    let _ = context;
    let mut labels = HashSet::new();
    for stmt in statements {
        if let Stmt::Label { name, span } = stmt {
            let key = key(name);
            if !labels.insert(key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!("Label '{}' is already declared", name),
                    Some(*span),
                ));
            }
        }
    }
    for stmt in statements {
        match stmt {
            Stmt::GoTo { label, span }
            | Stmt::Resume {
                target: ResumeTarget::Label(label),
                span,
            }
            | Stmt::OnError {
                mode: OnErrorMode::GoToLabel(label),
                span,
            } if !labels.contains(&key(label)) => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Label '{}' is not declared", label),
                    Some(*span),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_sub_call(
    name: &str,
    args: &[Expr],
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<(), Diagnostic> {
    let effective_name = if let Some(stripped) = name.strip_prefix("VBA.") {
        stripped
    } else {
        name
    };

    let builtin_subs = ["Randomize", "CallByName", "Kill", "MkDir", "RmDir", "ChDir"];
    if builtin_subs
        .iter()
        .any(|builtin| effective_name.eq_ignore_ascii_case(builtin))
    {
        for arg in args {
            validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.option_explicit,
            )?;
        }
        return Ok(());
    }

    let mut sub = validation
        .signatures
        .subs
        .get(&key(effective_name))
        .cloned();
    if sub.is_none()
        && let Some(owner_name) = validation.context.current_class()
        && let Some(class_sig) = validation.types.get_class(owner_name)
    {
        let member_key = key(effective_name);
        if let Some(sub_sig) = class_sig.subs.get(&member_key)
            && (sub_sig.is_shared || validation.symbols.contains_key("me"))
        {
            sub = Some(sub_sig.clone());
        }
    }

    let Some(sub) = sub else {
        if let Some(func) = validation.signatures.functions.get(&key(effective_name)) {
            if !func.is_declare {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!(
                        "Function '{}' cannot be called as a statement",
                        effective_name
                    ),
                    Some(span),
                ));
            }
            validate_arguments("Function", func, args, span, validation)?;
        } else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Sub '{}' is not defined", effective_name),
                Some(span),
            ));
        }
        return Ok(());
    };

    validate_arguments("Sub", &sub, args, span, validation)
}

fn class_field_object_type(
    object: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    context: &Context<'_>,
) -> Option<TypeName> {
    let ExprKind::Variable(name) = &object.kind else {
        return None;
    };
    if symbols.contains_key(&key(name)) {
        return None;
    }
    let class_name = context.current_class()?;
    let class_sig = types.get_class(class_name)?;
    class_sig
        .fields
        .get(&key(name))
        .map(|field| field.ty.clone())
}

fn class_field_expr_type(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    context: &Context<'_>,
) -> Option<TypeName> {
    let ExprKind::Variable(name) = &expr.kind else {
        return None;
    };
    if symbols.contains_key(&key(name)) {
        return None;
    }
    let class_name = context.current_class()?;
    let class_sig = types.get_class(class_name)?;
    class_sig
        .fields
        .get(&key(name))
        .map(|field| field.ty.clone())
}

fn get_redim_target_array_info(
    target: &ReDimTarget,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<Option<bool>, Diagnostic> {
    match target {
        ReDimTarget::Variable { name, span } => {
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                return match var_type {
                    VarType::Array(_, _, is_dynamic) => Ok(Some(is_dynamic)),
                    VarType::Scalar(_, TypeName::Variant)
                    | VarType::Optional(_, TypeName::Variant)
                    | VarType::Const(_, TypeName::Variant) => Ok(Some(true)), // Variant can hold dynamic array
                    _ => Ok(None),
                };
            }
            let Some(class_name) = context.current_class() else {
                if !option_explicit {
                    return Ok(Some(true));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            let class_sig = types
                .get_class(class_name)
                .expect("current class validated");
            let Some(field_sig) = class_sig.fields.get(&key(name)) else {
                if !option_explicit {
                    return Ok(Some(true));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            if field_sig.ty.same_type(&TypeName::Variant) {
                return Ok(Some(true));
            }
            Ok(field_sig
                .array
                .as_ref()
                .map(|a| matches!(a, ArrayDecl::Dynamic)))
        }
        ReDimTarget::Member {
            object,
            field,
            span,
        } => {
            let object_type =
                validate_expr(object, symbols, types, signatures, context, option_explicit)?;
            let _member_type =
                member_read_type(&object_type, field, types, *span, context.current_class())?;
            if let TypeName::User(class_name) = &object_type
                && let Some(class_sig) = types.get_class(class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(field))
            {
                if field_sig.ty.same_type(&TypeName::Variant) {
                    return Ok(Some(true));
                }
                return Ok(field_sig
                    .array
                    .as_ref()
                    .map(|a| matches!(a, ArrayDecl::Dynamic)));
            }
            Ok(Some(true)) // Assume dynamic if late-bound or Variant
        }
    }
}

fn stmt_span(stmt: &Stmt, _context: &Context<'_>) -> crate::runtime::Span {
    match stmt {
        Stmt::Dim { span, .. }
        | Stmt::DimMany { span, .. }
        | Stmt::Static { span, .. }
        | Stmt::StaticMany { span, .. }
        | Stmt::Const { span, .. }
        | Stmt::ConstMany { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::SetAssign { span, .. }
        | Stmt::ConsoleCall { span, .. }
        | Stmt::SubCall { span, .. }
        | Stmt::MemberSubCall { span, .. }
        | Stmt::RaiseEvent { span, .. }
        | Stmt::AddHandler { span, .. }
        | Stmt::RemoveHandler { span, .. }
        | Stmt::Await { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::If { span, .. }
        | Stmt::SelectCase { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoLoop { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForEach { span, .. }
        | Stmt::ReDim { span, .. }
        | Stmt::Erase { span, .. }
        | Stmt::LSet { span, .. }
        | Stmt::RSet { span, .. }
        | Stmt::Label { span, .. }
        | Stmt::GoTo { span, .. }
        | Stmt::OnError { span, .. }
        | Stmt::Resume { span, .. }
        | Stmt::With { span, .. }
        | Stmt::Using { span, .. }
        | Stmt::Exit { span, .. }
        | Stmt::TryCatch { span, .. }
        | Stmt::DebugPrint { span, .. }
        | Stmt::OpenFile { span, .. }
        | Stmt::CloseFile { span, .. }
        | Stmt::LineInput { span, .. }
        | Stmt::InputFile { span, .. }
        | Stmt::PrintFile { span, .. }
        | Stmt::WriteFile { span, .. }
        | Stmt::GetFile { span, .. }
        | Stmt::PutFile { span, .. }
        | Stmt::SeekFile { span, .. }
        | Stmt::NameFile { span, .. }
        | Stmt::Yield { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::End { span } => *span,
    }
}

fn stmt_uses_with_target(stmt: &Stmt, _context: &Context<'_>) -> bool {
    match stmt {
        Stmt::End { .. } | Stmt::Throw { .. } => false,
        Stmt::Const { value, .. } | Stmt::Return { expr: value, .. } => {
            expr_uses_with_target(value, _context)
        }
        Stmt::ConstMany { consts, .. } => consts
            .iter()
            .any(|const_decl| expr_uses_with_target(&const_decl.value, _context)),
        Stmt::Assign { target, expr, .. }
        | Stmt::SetAssign { target, expr, .. }
        | Stmt::LSet { target, expr, .. }
        | Stmt::RSet { target, expr, .. } => {
            assign_target_uses_with_target(target, _context)
                || expr_uses_with_target(expr, _context)
        }
        Stmt::ConsoleCall { args, .. }
        | Stmt::SubCall { args, .. }
        | Stmt::DebugPrint { args, .. } => {
            args.iter().any(|arg| expr_uses_with_target(arg, _context))
        }
        Stmt::OpenFile {
            path,
            number,
            record_len,
            ..
        } => {
            expr_uses_with_target(path, _context)
                || expr_uses_with_target(number, _context)
                || record_len
                    .as_ref()
                    .is_some_and(|expr| expr_uses_with_target(expr, _context))
        }
        Stmt::CloseFile { numbers, .. } => numbers
            .iter()
            .any(|number| expr_uses_with_target(number, _context)),
        Stmt::LineInput { number, target, .. } => {
            expr_uses_with_target(number, _context)
                || assign_target_uses_with_target(target, _context)
        }
        Stmt::InputFile {
            number, targets, ..
        } => {
            expr_uses_with_target(number, _context)
                || targets
                    .iter()
                    .any(|target| assign_target_uses_with_target(target, _context))
        }
        Stmt::PrintFile { number, items, .. } => {
            expr_uses_with_target(number, _context)
                || items
                    .iter()
                    .any(|item| expr_uses_with_target(&item.expr, _context))
        }
        Stmt::WriteFile { number, args, .. } => {
            expr_uses_with_target(number, _context)
                || args.iter().any(|arg| expr_uses_with_target(arg, _context))
        }
        Stmt::GetFile {
            number,
            position,
            target,
            ..
        } => {
            expr_uses_with_target(number, _context)
                || position
                    .as_ref()
                    .is_some_and(|expr| expr_uses_with_target(expr, _context))
                || assign_target_uses_with_target(target, _context)
        }
        Stmt::PutFile {
            number,
            position,
            expr,
            ..
        } => {
            expr_uses_with_target(number, _context)
                || position
                    .as_ref()
                    .is_some_and(|expr| expr_uses_with_target(expr, _context))
                || expr_uses_with_target(expr, _context)
        }
        Stmt::SeekFile {
            number, position, ..
        } => expr_uses_with_target(number, _context) || expr_uses_with_target(position, _context),
        Stmt::NameFile {
            old_path, new_path, ..
        } => expr_uses_with_target(old_path, _context) || expr_uses_with_target(new_path, _context),
        Stmt::MemberSubCall { object, args, .. } => {
            expr_uses_with_target(object, _context)
                || args.iter().any(|arg| expr_uses_with_target(arg, _context))
        }
        Stmt::RaiseEvent { args, .. } => {
            args.iter().any(|arg| expr_uses_with_target(arg, _context))
        }
        Stmt::AddHandler { event, handler, .. } | Stmt::RemoveHandler { event, handler, .. } => {
            expr_uses_with_target(event, _context) || expr_uses_with_target(handler, _context)
        }
        Stmt::Await { expr, .. } => expr_uses_with_target(expr, _context),
        Stmt::If {
            condition,
            then_body,
            elseif_branches,
            else_body,
            ..
        } => {
            expr_uses_with_target(condition, _context)
                || then_body.iter().any(|s| stmt_uses_with_target(s, _context))
                || elseif_branches.iter().any(|branch| {
                    expr_uses_with_target(&branch.condition, _context)
                        || branch
                            .body
                            .iter()
                            .any(|s| stmt_uses_with_target(s, _context))
                })
                || else_body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::SelectCase {
            subject,
            branches,
            else_body,
            ..
        } => {
            expr_uses_with_target(subject, _context)
                || branches.iter().any(|branch| {
                    branch
                        .items
                        .iter()
                        .any(|item| case_item_uses_with_target(item, _context))
                        || branch
                            .body
                            .iter()
                            .any(|s| stmt_uses_with_target(s, _context))
                })
                || else_body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::While {
            condition, body, ..
        } => {
            expr_uses_with_target(condition, _context)
                || body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::DoLoop {
            condition, body, ..
        } => {
            do_condition_uses_with_target(condition, _context)
                || body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::For {
            start,
            end,
            step,
            body,
            ..
        } => {
            expr_uses_with_target(start, _context)
                || expr_uses_with_target(end, _context)
                || step
                    .as_ref()
                    .is_some_and(|arg| expr_uses_with_target(arg, _context))
                || body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::ForEach { iterable, body, .. } => {
            expr_uses_with_target(iterable, _context)
                || body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::ReDim { dims, .. } => dims.iter().any(|(l, u)| {
            l.as_ref()
                .is_some_and(|arg| expr_uses_with_target(arg, _context))
                || expr_uses_with_target(u, _context)
        }),
        Stmt::TryCatch {
            try_body,
            catch_block,
            finally_body,
            ..
        } => {
            try_body.iter().any(|s| stmt_uses_with_target(s, _context))
                || catch_block
                    .as_ref()
                    .is_some_and(|c| c.body.iter().any(|s| stmt_uses_with_target(s, _context)))
                || finally_body
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_uses_with_target(s, _context)))
        }
        Stmt::Erase { .. } => false,
        Stmt::Label { .. } | Stmt::GoTo { .. } | Stmt::OnError { .. } | Stmt::Resume { .. } => {
            false
        }
        Stmt::With { target, .. } => expr_uses_with_target(target, _context),
        Stmt::Using { resource, body, .. } => {
            let resource_uses_with = match resource {
                UsingResource::Declaration(decl) => {
                    decl.initializer
                        .as_ref()
                        .is_some_and(|arg| expr_uses_with_target(arg, _context))
                        || decl
                            .new_args
                            .iter()
                            .any(|arg| expr_uses_with_target(arg, _context))
                }
                UsingResource::Target(expr) => expr_uses_with_target(expr, _context),
            };
            resource_uses_with || body.iter().any(|s| stmt_uses_with_target(s, _context))
        }
        Stmt::Dim { initializer, .. } | Stmt::Static { initializer, .. } => initializer
            .as_ref()
            .is_some_and(|arg| expr_uses_with_target(arg, _context)),
        Stmt::DimMany { decls, .. } | Stmt::StaticMany { decls, .. } => decls.iter().any(|decl| {
            decl.initializer
                .as_ref()
                .is_some_and(|arg| expr_uses_with_target(arg, _context))
        }),
        Stmt::Exit { .. } => false,
        Stmt::Yield { expr, .. } => expr_uses_with_target(expr, _context),
    }
}

fn validate_for_each_iterable_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<TypeName, Diagnostic> {
    if let Ok(element_type) =
        validate_array_expr(expr, symbols, types, signatures, context, option_explicit)
    {
        return Ok(element_type);
    }

    let iterable_type = validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
    match iterable_type {
        TypeName::Variant => Ok(TypeName::Variant),
        TypeName::User(class_name) => {
            let Some(class_sig) = types.get_class(&class_name) else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!("For Each target '{}' is not enumerable", class_name),
                    Some(expr.span),
                ));
            };
            if class_sig.iterator.is_some() || class_sig.enumerator.is_some() {
                Ok(TypeName::Variant)
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!(
                        "Class '{}' is not enumerable; define an Iterator or a VB_UserMemId = -4 _NewEnum member",
                        class_sig.name
                    ),
                    Some(expr.span),
                ))
            }
        }
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "For Each requires an array, Variant array, or enumerable object",
            Some(expr.span),
        )),
    }
}

fn assign_target_uses_with_target(target: &AssignTarget, _context: &Context<'_>) -> bool {
    match target {
        AssignTarget::Variable { .. } => false,
        AssignTarget::ArrayElement { indices, .. } => indices
            .iter()
            .any(|arg| expr_uses_with_target(arg, _context)),
        AssignTarget::Member { object, .. } => expr_uses_with_target(object, _context),
        AssignTarget::MemberArrayElement {
            object, indices, ..
        } => {
            expr_uses_with_target(object, _context)
                || indices
                    .iter()
                    .any(|arg| expr_uses_with_target(arg, _context))
        }
    }
}

fn case_item_uses_with_target(item: &CaseItem, _context: &Context<'_>) -> bool {
    match item {
        CaseItem::Value(expr) | CaseItem::Compare { expr, .. } => {
            expr_uses_with_target(expr, _context)
        }
        CaseItem::Range { start, end } => {
            expr_uses_with_target(start, _context) || expr_uses_with_target(end, _context)
        }
    }
}

fn do_condition_uses_with_target(condition: &DoLoopCondition, _context: &Context<'_>) -> bool {
    match condition {
        DoLoopCondition::Infinite => false,
        DoLoopCondition::PreWhile(expr)
        | DoLoopCondition::PreUntil(expr)
        | DoLoopCondition::PostWhile(expr)
        | DoLoopCondition::PostUntil(expr) => expr_uses_with_target(expr, _context),
    }
}

fn expr_uses_with_target(expr: &Expr, _context: &Context<'_>) -> bool {
    match &expr.kind {
        ExprKind::WithTarget => true,
        ExprKind::New {
            args, initializer, ..
        } => {
            args.iter().any(|arg| expr_uses_with_target(arg, _context))
                || initializer
                    .as_ref()
                    .is_some_and(|init| init.iter().any(|arg| expr_uses_with_target(arg, _context)))
        }
        ExprKind::Call { args, .. } => args.iter().any(|arg| expr_uses_with_target(arg, _context)),
        ExprKind::Index { target, args } => {
            expr_uses_with_target(target, _context)
                || args.iter().any(|arg| expr_uses_with_target(arg, _context))
        }
        ExprKind::IIf {
            condition,
            true_expr,
            false_expr,
        } => {
            expr_uses_with_target(condition, _context)
                || expr_uses_with_target(true_expr, _context)
                || expr_uses_with_target(false_expr, _context)
        }
        ExprKind::NamedArg { expr, .. } | ExprKind::TypeOfIs { expr, .. } => {
            expr_uses_with_target(expr, _context)
        }
        ExprKind::MemberAccess { object, .. } => expr_uses_with_target(object, _context),
        ExprKind::MemberCall { object, args, .. } => {
            expr_uses_with_target(object, _context)
                || args.iter().any(|arg| expr_uses_with_target(arg, _context))
        }
        ExprKind::Binary { left, right, .. } => {
            expr_uses_with_target(left, _context) || expr_uses_with_target(right, _context)
        }
        ExprKind::Unary { expr, .. } => expr_uses_with_target(expr, _context),
        ExprKind::Lambda { body, .. } => expr_uses_with_target(body, _context),
        ExprKind::Await(expr) => expr_uses_with_target(expr, _context),
        ExprKind::AddressOf(inner) => expr_uses_with_target(inner, _context),
        ExprKind::String(_)
        | ExprKind::Integer(_)
        | ExprKind::Long(_)
        | ExprKind::LongLong(_)
        | ExprKind::Single(_)
        | ExprKind::Double(_)
        | ExprKind::Currency(_)
        | ExprKind::Decimal(_)
        | ExprKind::Boolean(_)
        | ExprKind::DateLiteral(_)
        | ExprKind::Nothing
        | ExprKind::Empty
        | ExprKind::Null
        | ExprKind::Missing
        | ExprKind::Me
        | ExprKind::MyBase
        | ExprKind::MyClass
        | ExprKind::Variable(_) => false,
        ExprKind::PassingModeOverride { expr, .. } => expr_uses_with_target(expr, _context),
    }
}
