use super::*;
use crate::UsingResource;
use std::collections::HashSet;

fn validate_const_decl(
    name: &str,
    ty: &Option<TypeName>,
    value: &Expr,
    span: crate::runtime::Span,
    symbols: &mut HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    ensure_const_expr(value, symbols, types)?;
    let value_type = validate_expr(value, symbols, types, signatures)?;
    let const_type = ty.clone().unwrap_or(value_type.clone());
    ensure_known_type(&const_type, types, span)?;
    ensure_assignable_expr(&const_type, &value_type, value, types, span)?;
    let key = key(name);
    if symbols.contains_key(&key) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
            format!("Variable '{}' is already declared", name),
            Some(span),
        ));
    }
    symbols.insert(key, VarType::Const(const_type));
    Ok(())
}

pub(super) fn validate_statements(
    statements: &[Stmt],
    symbols: &mut HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    mut context: Context<'_>,
    loop_context: LoopContext,
    in_with: bool,
) -> Result<(), Diagnostic> {
    validate_labels(statements)?;
    for stmt in statements {
        if !in_with && stmt_uses_with_target(stmt) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Dot member access requires an active With block",
                Some(stmt_span(stmt)),
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
                let ty =
                    declared_variable_type(ty, initializer, symbols, types, signatures, *span)?;
                ensure_known_type(&ty, types, *span)?;
                validate_as_new(*as_new, &ty, new_args, symbols, types, signatures, *span)?;
                if let Some(initializer) = initializer {
                    if array.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Array declarations cannot use an initializer",
                            Some(initializer.span),
                        ));
                    }
                    let source_type = validate_expr(initializer, symbols, types, signatures)?;
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
                let var_type = if array.is_some() {
                    VarType::Array(ty)
                } else {
                    VarType::Scalar(ty)
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
                        symbols,
                        types,
                        signatures,
                        decl.span,
                    )?;
                    ensure_known_type(&ty, types, decl.span)?;
                    validate_as_new(
                        decl.as_new,
                        &ty,
                        &decl.new_args,
                        symbols,
                        types,
                        signatures,
                        decl.span,
                    )?;
                    if let Some(initializer) = &decl.initializer {
                        if decl.array.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                "Array declarations cannot use an initializer",
                                Some(initializer.span),
                            ));
                        }
                        let source_type = validate_expr(initializer, symbols, types, signatures)?;
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
                    let var_type = if decl.array.is_some() {
                        VarType::Array(ty)
                    } else {
                        VarType::Scalar(ty)
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
                validate_const_decl(name, ty, value, *span, symbols, types, signatures)?;
            }
            Stmt::ConstMany { consts, .. } => {
                for const_decl in consts {
                    validate_const_decl(
                        &const_decl.name,
                        &const_decl.ty,
                        &const_decl.value,
                        const_decl.span,
                        symbols,
                        types,
                        signatures,
                    )?;
                }
            }
            Stmt::Assign { target, expr, span } => {
                let expr_type = class_field_expr_type(expr, symbols, types, &context)
                    .map(Ok)
                    .unwrap_or_else(|| validate_expr(expr, symbols, types, signatures))?;
                let target_type = validate_assignment_target(
                    target, &expr_type, symbols, types, signatures, &context,
                )?;
                ensure_assignable_expr(&target_type, &expr_type, expr, types, *span)?;
            }
            Stmt::SetAssign { target, expr, span } => {
                let expr_type = class_field_expr_type(expr, symbols, types, &context)
                    .map(Ok)
                    .unwrap_or_else(|| validate_expr(expr, symbols, types, signatures))?;
                let target_type = validate_assignment_target(
                    target, &expr_type, symbols, types, signatures, &context,
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
            Stmt::ConsoleWriteLine { args, .. } => {
                for arg in args {
                    validate_expr(arg, symbols, types, signatures)?;
                }
            }
            Stmt::SubCall { name, args, span } => {
                validate_sub_call(name, args, *span, symbols, types, signatures)?;
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
                    validate_sub_call(method, args, *span, symbols, types, signatures)?;
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
                                    args: args.clone(),
                                },
                                span: *span,
                            },
                            symbols,
                            types,
                            signatures,
                        )?;
                        continue;
                    }
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "Err only supports Clear() and Raise()",
                        Some(*span),
                    ));
                }
                let object_type = class_field_object_type(object, symbols, types, &context)
                    .map(Ok)
                    .unwrap_or_else(|| validate_expr(object, symbols, types, signatures))?;
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
                validate_arguments("Event", event_sig, args, symbols, types, signatures, *span)?;
            }
            Stmt::Return { expr, span } => match &mut context {
                Context::Sub | Context::MethodSub { .. } | Context::PropertyLetSet { .. } => {
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
                    let expr_type = validate_expr(expr, symbols, types, signatures)?;
                    ensure_assignable_expr(return_type, &expr_type, expr, types, *span)?;
                    **saw_return = true;
                }
            },
            Stmt::Yield { expr, span } => match &mut context {
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
                    let expr_type = validate_expr(expr, symbols, types, signatures)?;
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
            },
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                ..
            } => {
                ensure_assignable(
                    &TypeName::Boolean,
                    &validate_expr(condition, symbols, types, signatures)?,
                    condition.span,
                )?;
                validate_statements(
                    then_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                    in_with,
                )?;
                for branch in elseif_branches {
                    ensure_assignable(
                        &TypeName::Boolean,
                        &validate_expr(&branch.condition, symbols, types, signatures)?,
                        branch.condition.span,
                    )?;
                    validate_statements(
                        &branch.body,
                        symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                        in_with,
                    )?;
                }
                validate_statements(
                    else_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                    in_with,
                )?;
            }
            Stmt::SelectCase {
                subject,
                branches,
                else_body,
                ..
            } => {
                let subject_type = validate_expr(subject, symbols, types, signatures)?;
                for branch in branches {
                    for item in &branch.items {
                        validate_case_item(item, &subject_type, symbols, types, signatures)?;
                    }
                    validate_statements(
                        &branch.body,
                        symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                        in_with,
                    )?;
                }
                validate_statements(
                    else_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                    in_with,
                )?;
            }
            Stmt::While {
                condition, body, ..
            } => {
                validate_expr(condition, symbols, types, signatures)?;
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_while(),
                    in_with,
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
                            &validate_expr(condition, symbols, types, signatures)?,
                            condition.span,
                        )?;
                    }
                }
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_do(),
                    in_with,
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
                    &validate_expr(start, symbols, types, signatures)?,
                    start.span,
                )?;
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(end, symbols, types, signatures)?,
                    end.span,
                )?;
                if let Some(step) = step {
                    ensure_assignable(
                        &TypeName::Integer,
                        &validate_expr(step, symbols, types, signatures)?,
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
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_for(),
                    in_with,
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
                let array_type =
                    validate_for_each_iterable_expr(iterable, symbols, types, signatures)?;
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
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context.in_for(),
                    in_with,
                )?;
            }
            Stmt::ReDim {
                target,
                dims,
                preserve,
                span,
                ..
            } => {
                if !redim_target_is_dynamic_array(target, symbols, types, signatures, &context)? {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "ReDim target must be a dynamic array",
                        Some(*span),
                    ));
                }
                for (lower, upper) in dims {
                    let upper_type = class_field_expr_type(upper, symbols, types, &context)
                        .map(Ok)
                        .unwrap_or_else(|| validate_expr(upper, symbols, types, signatures))?;
                    ensure_assignable(&TypeName::Integer, &upper_type, upper.span)?;
                    if let Some(lower) = lower {
                        let lower_type = class_field_expr_type(lower, symbols, types, &context)
                            .map(Ok)
                            .unwrap_or_else(|| validate_expr(lower, symbols, types, signatures))?;
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
                if !redim_target_is_dynamic_array(target, symbols, types, signatures, &context)? {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "Erase target must be an array or Variant",
                        Some(*span),
                    ));
                }
            }
            Stmt::Label { .. } => {}
            Stmt::GoTo { .. } => {}
            Stmt::OnError { .. } => {}
            Stmt::Resume { .. } => {}
            Stmt::With { target, body, .. } => {
                validate_expr(target, symbols, types, signatures)?;
                validate_statements(
                    body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                    true,
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
                        symbols,
                        types,
                        signatures,
                        decl.span,
                    )?;
                    ensure_known_type(&ty, types, decl.span)?;
                    validate_as_new(
                        decl.as_new,
                        &ty,
                        &decl.new_args,
                        symbols,
                        types,
                        signatures,
                        decl.span,
                    )?;
                    if let Some(initializer) = &decl.initializer {
                        let source_type = validate_expr(initializer, symbols, types, signatures)?;
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
                    using_symbols.insert(decl_key, VarType::Scalar(ty));
                    validate_statements(
                        body,
                        &mut using_symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                        in_with,
                    )?;
                }
                UsingResource::Target(expr) => {
                    let ty = validate_expr(expr, symbols, types, signatures)?;
                    validate_using_disposable(&ty, types, *span)?;
                    validate_statements(
                        body,
                        symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                        in_with,
                    )?;
                }
            },
            Stmt::Exit { target, span } => {
                validate_exit(*target, *span, &context, loop_context)?;
            }
            Stmt::TryCatch {
                try_body,
                catch_block,
                finally_body,
                ..
            } => {
                validate_statements(
                    try_body,
                    symbols,
                    types,
                    signatures,
                    context.reborrow(),
                    loop_context,
                    in_with,
                )?;
                if let Some(catch) = catch_block {
                    let mut catch_symbols = symbols.clone();
                    if let Some(var_name) = &catch.variable {
                        catch_symbols.insert(
                            key(var_name),
                            VarType::Scalar(TypeName::User("Error".to_string())),
                        );
                    }
                    validate_statements(
                        &catch.body,
                        &mut catch_symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                        in_with,
                    )?;
                }
                if let Some(finally_body) = finally_body {
                    validate_statements(
                        finally_body,
                        symbols,
                        types,
                        signatures,
                        context.reborrow(),
                        loop_context,
                        in_with,
                    )?;
                }
            }
            Stmt::DebugPrint { args, .. } => {
                for arg in args {
                    validate_expr(arg, symbols, types, signatures)?;
                }
            }
        }
    }

    Ok(())
}

fn declared_variable_type(
    ty: &Option<TypeName>,
    initializer: &Option<Expr>,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
) -> Result<TypeName, Diagnostic> {
    if let Some(ty) = ty {
        return Ok(ty.clone());
    }
    if let Some(initializer) = initializer {
        return validate_expr(initializer, symbols, types, signatures);
    }
    let _ = span;
    Ok(TypeName::Variant)
}

fn validate_as_new(
    as_new: bool,
    ty: &TypeName,
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if !as_new {
        return Ok(());
    }
    let TypeName::User(class_name) = ty else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "As New requires a class type",
            Some(span),
        ));
    };
    let expr = Expr {
        kind: ExprKind::New {
            class_name: class_name.clone(),
            args: args.to_vec(),
        },
        span,
    };
    validate_expr(&expr, symbols, types, signatures).map(|_| ())
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

fn validate_labels(statements: &[Stmt]) -> Result<(), Diagnostic> {
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
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    let effective_name = if let Some(stripped) = name.strip_prefix("VBA.") {
        stripped
    } else {
        name
    };

    let builtin_subs = ["Randomize", "CallByName"];
    if builtin_subs
        .iter()
        .any(|builtin| effective_name.eq_ignore_ascii_case(builtin))
    {
        for arg in args {
            validate_expr(arg, symbols, types, signatures)?;
        }
        return Ok(());
    }

    let Some(sub) = signatures.subs.get(&key(effective_name)) else {
        if let Some(func) = signatures.functions.get(&key(effective_name)) {
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
            validate_arguments("Function", func, args, symbols, types, signatures, span)?;
        } else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Sub '{}' is not defined", effective_name),
                Some(span),
            ));
        }
        return Ok(());
    };

    validate_arguments("Sub", sub, args, symbols, types, signatures, span)
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

fn redim_target_is_dynamic_array(
    target: &ReDimTarget,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
) -> Result<bool, Diagnostic> {
    match target {
        ReDimTarget::Variable { name, span } => {
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                return Ok(matches!(var_type, VarType::Array(_)));
            }
            let Some(class_name) = context.current_class() else {
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
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            Ok(matches!(field_sig.array, Some(ArrayDecl::Dynamic))
                || field_sig.ty.same_type(&TypeName::Variant))
        }
        ReDimTarget::Member {
            object,
            field,
            span,
        } => {
            let object_type = validate_expr(object, symbols, types, signatures)?;
            let member_type =
                member_read_type(&object_type, field, types, *span, context.current_class())?;
            if let TypeName::User(class_name) = &object_type
                && let Some(class_sig) = types.get_class(class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(field))
            {
                return Ok(matches!(field_sig.array, Some(ArrayDecl::Dynamic))
                    || field_sig.ty.same_type(&TypeName::Variant));
            }
            Ok(member_type.same_type(&TypeName::Variant))
        }
    }
}

fn stmt_span(stmt: &Stmt) -> crate::runtime::Span {
    match stmt {
        Stmt::Dim { span, .. }
        | Stmt::DimMany { span, .. }
        | Stmt::Static { span, .. }
        | Stmt::StaticMany { span, .. }
        | Stmt::Const { span, .. }
        | Stmt::ConstMany { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::SetAssign { span, .. }
        | Stmt::ConsoleWriteLine { span, .. }
        | Stmt::SubCall { span, .. }
        | Stmt::MemberSubCall { span, .. }
        | Stmt::RaiseEvent { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::If { span, .. }
        | Stmt::SelectCase { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoLoop { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForEach { span, .. }
        | Stmt::ReDim { span, .. }
        | Stmt::Erase { span, .. }
        | Stmt::Label { span, .. }
        | Stmt::GoTo { span, .. }
        | Stmt::OnError { span, .. }
        | Stmt::Resume { span, .. }
        | Stmt::With { span, .. }
        | Stmt::Using { span, .. }
        | Stmt::Exit { span, .. }
        | Stmt::TryCatch { span, .. }
        | Stmt::DebugPrint { span, .. }
        | Stmt::Yield { span, .. } => *span,
    }
}

fn stmt_uses_with_target(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Const { value, .. } | Stmt::Return { expr: value, .. } => {
            expr_uses_with_target(value)
        }
        Stmt::ConstMany { consts, .. } => consts
            .iter()
            .any(|const_decl| expr_uses_with_target(&const_decl.value)),
        Stmt::Assign { target, expr, .. } | Stmt::SetAssign { target, expr, .. } => {
            assign_target_uses_with_target(target) || expr_uses_with_target(expr)
        }
        Stmt::ConsoleWriteLine { args, .. }
        | Stmt::SubCall { args, .. }
        | Stmt::DebugPrint { args, .. } => args.iter().any(expr_uses_with_target),
        Stmt::MemberSubCall { object, args, .. } => {
            expr_uses_with_target(object) || args.iter().any(expr_uses_with_target)
        }
        Stmt::RaiseEvent { args, .. } => args.iter().any(expr_uses_with_target),
        Stmt::If {
            condition,
            then_body,
            elseif_branches,
            else_body,
            ..
        } => {
            expr_uses_with_target(condition)
                || then_body.iter().any(stmt_uses_with_target)
                || elseif_branches.iter().any(|branch| {
                    expr_uses_with_target(&branch.condition)
                        || branch.body.iter().any(stmt_uses_with_target)
                })
                || else_body.iter().any(stmt_uses_with_target)
        }
        Stmt::SelectCase {
            subject,
            branches,
            else_body,
            ..
        } => {
            expr_uses_with_target(subject)
                || branches.iter().any(|branch| {
                    branch.items.iter().any(case_item_uses_with_target)
                        || branch.body.iter().any(stmt_uses_with_target)
                })
                || else_body.iter().any(stmt_uses_with_target)
        }
        Stmt::While {
            condition, body, ..
        } => expr_uses_with_target(condition) || body.iter().any(stmt_uses_with_target),
        Stmt::DoLoop {
            condition, body, ..
        } => do_condition_uses_with_target(condition) || body.iter().any(stmt_uses_with_target),
        Stmt::For {
            start,
            end,
            step,
            body,
            ..
        } => {
            expr_uses_with_target(start)
                || expr_uses_with_target(end)
                || step.as_ref().is_some_and(expr_uses_with_target)
                || body.iter().any(stmt_uses_with_target)
        }
        Stmt::ForEach { iterable, body, .. } => {
            expr_uses_with_target(iterable) || body.iter().any(stmt_uses_with_target)
        }
        Stmt::ReDim { dims, .. } => dims.iter().any(|(l, u)| {
            l.as_ref().is_some_and(expr_uses_with_target) || expr_uses_with_target(u)
        }),
        Stmt::TryCatch {
            try_body,
            catch_block,
            finally_body,
            ..
        } => {
            try_body.iter().any(stmt_uses_with_target)
                || catch_block
                    .as_ref()
                    .is_some_and(|c| c.body.iter().any(stmt_uses_with_target))
                || finally_body
                    .as_ref()
                    .is_some_and(|f| f.iter().any(stmt_uses_with_target))
        }
        Stmt::Erase { .. } => false,
        Stmt::Label { .. } | Stmt::GoTo { .. } | Stmt::OnError { .. } | Stmt::Resume { .. } => {
            false
        }
        Stmt::With { target, .. } => expr_uses_with_target(target),
        Stmt::Using { resource, body, .. } => {
            let resource_uses_with = match resource {
                UsingResource::Declaration(decl) => {
                    decl.initializer.as_ref().is_some_and(expr_uses_with_target)
                        || decl.new_args.iter().any(expr_uses_with_target)
                }
                UsingResource::Target(expr) => expr_uses_with_target(expr),
            };
            resource_uses_with || body.iter().any(stmt_uses_with_target)
        }
        Stmt::Dim { initializer, .. } | Stmt::Static { initializer, .. } => {
            initializer.as_ref().is_some_and(expr_uses_with_target)
        }
        Stmt::DimMany { decls, .. } | Stmt::StaticMany { decls, .. } => decls
            .iter()
            .any(|decl| decl.initializer.as_ref().is_some_and(expr_uses_with_target)),
        Stmt::Exit { .. } => false,
        Stmt::Yield { expr, .. } => expr_uses_with_target(expr),
    }
}

fn validate_for_each_iterable_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<TypeName, Diagnostic> {
    if let Ok(element_type) = validate_array_expr(expr, symbols, types, signatures) {
        return Ok(element_type);
    }

    let iterable_type = validate_expr(expr, symbols, types, signatures)?;
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

fn assign_target_uses_with_target(target: &AssignTarget) -> bool {
    match target {
        AssignTarget::Variable { .. } => false,
        AssignTarget::ArrayElement { indices, .. } => indices.iter().any(expr_uses_with_target),
        AssignTarget::Member { object, .. } => expr_uses_with_target(object),
        AssignTarget::MemberArrayElement {
            object, indices, ..
        } => expr_uses_with_target(object) || indices.iter().any(expr_uses_with_target),
    }
}

fn case_item_uses_with_target(item: &CaseItem) -> bool {
    match item {
        CaseItem::Value(expr) | CaseItem::Compare { expr, .. } => expr_uses_with_target(expr),
        CaseItem::Range { start, end } => {
            expr_uses_with_target(start) || expr_uses_with_target(end)
        }
    }
}

fn do_condition_uses_with_target(condition: &DoLoopCondition) -> bool {
    match condition {
        DoLoopCondition::Infinite => false,
        DoLoopCondition::PreWhile(expr)
        | DoLoopCondition::PreUntil(expr)
        | DoLoopCondition::PostWhile(expr)
        | DoLoopCondition::PostUntil(expr) => expr_uses_with_target(expr),
    }
}

fn expr_uses_with_target(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::WithTarget => true,
        ExprKind::New { args, .. } | ExprKind::Call { args, .. } => {
            args.iter().any(expr_uses_with_target)
        }
        ExprKind::Index { target, args } => {
            expr_uses_with_target(target) || args.iter().any(expr_uses_with_target)
        }
        ExprKind::IIf {
            condition,
            true_expr,
            false_expr,
        } => {
            expr_uses_with_target(condition)
                || expr_uses_with_target(true_expr)
                || expr_uses_with_target(false_expr)
        }
        ExprKind::NamedArg { expr, .. } | ExprKind::TypeOfIs { expr, .. } => {
            expr_uses_with_target(expr)
        }
        ExprKind::MemberAccess { object, .. } => expr_uses_with_target(object),
        ExprKind::MemberCall { object, args, .. } => {
            expr_uses_with_target(object) || args.iter().any(expr_uses_with_target)
        }
        ExprKind::Binary { left, right, .. } => {
            expr_uses_with_target(left) || expr_uses_with_target(right)
        }
        ExprKind::Unary { expr, .. } => expr_uses_with_target(expr),
        ExprKind::AddressOf(inner) => expr_uses_with_target(inner),
        ExprKind::String(_)
        | ExprKind::Integer(_)
        | ExprKind::Long(_)
        | ExprKind::LongLong(_)
        | ExprKind::Single(_)
        | ExprKind::Double(_)
        | ExprKind::Currency(_)
        | ExprKind::Decimal(_)
        | ExprKind::Boolean(_)
        | ExprKind::Nothing
        | ExprKind::Empty
        | ExprKind::Null
        | ExprKind::Missing
        | ExprKind::Me
        | ExprKind::Variable(_) => false,
        ExprKind::PassingModeOverride { expr, .. } => expr_uses_with_target(expr),
    }
}
