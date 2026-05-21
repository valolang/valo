use super::*;

pub(super) fn validate_assignment_target(
    target: &AssignTarget,
    value_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
) -> Result<TypeName, Diagnostic> {
    match target {
        AssignTarget::Variable { name, span } => {
            let target_type = if let Some(target_type) = symbols.get(&key(name)).cloned() {
                target_type
            } else if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    VarType::Scalar(field_sig.ty.clone())
                } else if let Some(type_sig) = types.get(owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    VarType::Scalar(field_sig.ty.clone())
                } else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", name),
                        Some(*span),
                    ));
                }
            } else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            if target_type.is_const() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                    format!("Constant '{}' cannot be assigned", name),
                    Some(*span),
                ));
            }
            let Some(target_type) = target_type.scalar_type() else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!("Array variable '{}' cannot be used as a scalar", name),
                    Some(*span),
                ));
            };
            Ok(target_type)
        }
        AssignTarget::ArrayElement {
            name,
            indices,
            span,
        } => {
            let var_type = if let Some(var_type) = symbols.get(&key(name)).cloned() {
                var_type
            } else if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.array.is_some() {
                        VarType::Array(field_sig.ty.clone())
                    } else if field_sig.ty.same_type(&TypeName::Variant) {
                        VarType::Scalar(TypeName::Variant)
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Variable '{}' is not an array", name),
                            Some(*span),
                        ));
                    }
                } else if let Some(type_sig) = types.get(owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    if field_sig.array.is_some() {
                        VarType::Array(field_sig.ty.clone())
                    } else if field_sig.ty.same_type(&TypeName::Variant) {
                        VarType::Scalar(TypeName::Variant)
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Variable '{}' is not an array", name),
                            Some(*span),
                        ));
                    }
                } else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", name),
                        Some(*span),
                    ));
                }
            } else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(*span),
                ));
            };
            let element_type = match var_type {
                VarType::Array(ty) => ty,
                VarType::Scalar(TypeName::User(class_name))
                | VarType::Optional(TypeName::User(class_name))
                | VarType::Const(TypeName::User(class_name))
                    if types
                        .get_class(&class_name)
                        .and_then(|class| class.default_property.as_ref())
                        .is_some() =>
                {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures)?;
                    }
                    return Ok(value_type.clone());
                }
                VarType::Scalar(TypeName::Variant)
                | VarType::Optional(TypeName::Variant)
                | VarType::Const(TypeName::Variant) => TypeName::Variant,
                _ => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        format!("Variable '{}' is not an array", name),
                        Some(*span),
                    ));
                }
            };
            for index in indices {
                ensure_assignable(
                    &TypeName::Integer,
                    &validate_expr(index, symbols, types, signatures)?,
                    index.span,
                )?;
            }
            Ok(element_type)
        }
        AssignTarget::Member {
            object,
            field,
            span,
        } => {
            let object_type = validate_expr(object, symbols, types, signatures)?;
            let current_class = member_access_class(object, &object_type)
                .or_else(|| context.current_class().map(str::to_string));
            member_assignment_type(
                &object_type,
                field,
                value_type,
                types,
                *span,
                current_class.as_deref(),
            )
        }
    }
}

pub(super) fn validate_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::String(_) => Ok(TypeName::String),
        ExprKind::Integer(value) => {
            let val = *value;
            if val >= i16::MIN as i64 && val <= i16::MAX as i64 {
                Ok(TypeName::Integer)
            } else if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                Ok(TypeName::Long)
            } else {
                Ok(TypeName::Int64)
            }
        }
        ExprKind::Double(_) => Ok(TypeName::Double),
        ExprKind::Boolean(_) => Ok(TypeName::Boolean),
        ExprKind::Nothing | ExprKind::Empty | ExprKind::Null => Ok(TypeName::Variant),
        ExprKind::Missing => Ok(TypeName::Variant),
        ExprKind::NamedArg { .. } => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "Named arguments are only valid inside call argument lists",
            Some(expr.span),
        )),
        ExprKind::TypeOfIs {
            expr: object,
            class_name,
        } => {
            let class = types.get_class(class_name).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Class '{}' is not defined", class_name),
                    Some(expr.span),
                )
            })?;
            let object_type = validate_expr(object, symbols, types, signatures)?;
            if is_object_reference_expr(object, &object_type, types) {
                Ok(TypeName::Boolean)
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "TypeOf requires a class object; '{}' is a class",
                        class.name
                    ),
                    Some(object.span),
                ))
            }
        }
        ExprKind::Me => match symbols.get("me").cloned() {
            Some(VarType::Scalar(ty)) | Some(VarType::Optional(ty)) | Some(VarType::Const(ty)) => {
                Ok(ty)
            }
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Me is only valid inside class methods",
                Some(expr.span),
            )),
        },
        ExprKind::WithTarget => Ok(TypeName::Variant),
        ExprKind::New { class_name, args } => {
            if let Some(type_sig) = types.get(class_name) {
                if !type_sig.is_structure {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Type '{}' cannot be constructed with New; use Structure",
                            type_sig.name
                        ),
                        Some(expr.span),
                    ));
                }
                if let Some(init) = type_sig.subs.get("initialize") {
                    validate_arguments("Sub", init, args, symbols, types, signatures, expr.span)?;
                } else if !args.is_empty() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Structure '{}' has no Constructor", type_sig.name),
                        Some(expr.span),
                    ));
                }
                return Ok(TypeName::User(type_sig.name.clone()));
            }
            let class_sig = types.get_class(class_name).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Class or Structure '{}' is not defined", class_name),
                    Some(expr.span),
                )
            })?;
            if let Some(init) = class_sig
                .subs
                .get("initialize")
                .or_else(|| class_sig.subs.get("class_initialize"))
            {
                validate_arguments("Sub", init, args, symbols, types, signatures, expr.span)?;
            } else if !args.is_empty() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Class '{}' has no Initialize constructor", class_sig.name),
                    Some(expr.span),
                ));
            }
            Ok(TypeName::User(class_sig.name.clone()))
        }
        ExprKind::Variable(name) => {
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                match var_type {
                    VarType::Scalar(ty) | VarType::Optional(ty) | VarType::Const(ty) => {
                        return Ok(ty);
                    }
                    VarType::Array(_) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Array variable '{}' cannot be used as a scalar", name),
                            Some(expr.span),
                        ));
                    }
                }
            }
            if name.eq_ignore_ascii_case("Err") {
                return Ok(TypeName::Variant);
            }
            if name.eq_ignore_ascii_case("Erl") {
                return Ok(TypeName::Integer);
            }
            if name.eq_ignore_ascii_case("Console") {
                return Ok(TypeName::Variant);
            }
            if name.eq_ignore_ascii_case("VBA") {
                return Ok(TypeName::Variant);
            }
            let builtins = [
                "vbBinaryCompare",
                "vbTextCompare",
                "vbEmpty",
                "vbNull",
                "vbInteger",
                "vbLong",
                "vbSingle",
                "vbDouble",
                "vbCurrency",
                "vbDate",
                "vbString",
                "vbObject",
                "vbError",
                "vbBoolean",
                "vbVariant",
                "vbDataObject",
                "vbDecimal",
                "vbByte",
                "vbLongLong",
                "vbUserDefinedType",
                "vbArray",
                "VbMethod",
                "VbGet",
                "VbLet",
                "VbSet",
            ];
            if builtins.iter().any(|b| name.eq_ignore_ascii_case(b)) {
                return Ok(TypeName::Integer);
            }
            if let Some(function) = signatures.functions.get(&key(name)) {
                validate_arguments(
                    "Function",
                    function,
                    &[],
                    symbols,
                    types,
                    signatures,
                    expr.span,
                )?;
                return Ok(function.return_type.clone().expect("function return type"));
            }
            if let Some(VarType::Scalar(TypeName::User(owner_name))) = symbols.get("me").cloned() {
                if let Some(class_sig) = types.get_class(&owner_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.array.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Array variable '{}' cannot be used as a scalar", name),
                            Some(expr.span),
                        ));
                    }
                    return Ok(field_sig.ty.clone());
                }
                if let Some(type_sig) = types.get(&owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    if field_sig.array.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Array variable '{}' cannot be used as a scalar", name),
                            Some(expr.span),
                        ));
                    }
                    return Ok(field_sig.ty.clone());
                }
            }
            if enum_member_value_type(name, types).is_some() {
                Ok(TypeName::Integer)
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(expr.span),
                ))
            }
        }
        ExprKind::MemberAccess { object, field } => {
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case("Err")
            {
                if field.eq_ignore_ascii_case("Number") {
                    return Ok(TypeName::Integer);
                }
                if field.eq_ignore_ascii_case("Description")
                    || field.eq_ignore_ascii_case("Source")
                    || field.eq_ignore_ascii_case("HelpFile")
                {
                    return Ok(TypeName::String);
                }
                if field.eq_ignore_ascii_case("HelpContext") {
                    return Ok(TypeName::Integer);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Err has no member '{}'", field),
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(enum_name) = &object.kind
                && let Some(enum_sig) = types.get_enum(enum_name)
            {
                if enum_sig.members.contains_key(&key(field)) {
                    return Ok(TypeName::Integer);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Enum '{}' has no member '{}'", enum_sig.name, field),
                    Some(expr.span),
                ));
            }
            let object_type = validate_expr(object, symbols, types, signatures)?;
            let current_class = member_access_class(object, &object_type);
            member_read_type(
                &object_type,
                field,
                types,
                expr.span,
                current_class.as_deref(),
            )
        }
        ExprKind::MemberCall {
            object,
            method,
            args,
        } => {
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case("VBA")
            {
                if let Some(ty) =
                    validate_builtin_function(method, args, symbols, types, signatures, expr.span)?
                {
                    return Ok(ty);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Module 'VBA' has no member '{}'", method),
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case("Err")
            {
                if method.eq_ignore_ascii_case("Clear") && args.is_empty() {
                    return Ok(TypeName::Variant);
                }
                if method.eq_ignore_ascii_case("Raise") {
                    validate_err_raise_args(args, symbols, types, signatures, expr.span)?;
                    return Ok(TypeName::Variant);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Err only supports Clear() and Raise()",
                    Some(expr.span),
                ));
            }
            let object_type = validate_expr(object, symbols, types, signatures)?;
            validate_method_call(
                &object_type,
                method,
                args,
                true,
                expr.span,
                symbols,
                types,
                signatures,
                member_access_class(object, &object_type).as_deref(),
            )
        }
        ExprKind::Call { name, args } => {
            if let Some(ty) =
                validate_builtin_function(name, args, symbols, types, signatures, expr.span)?
            {
                return Ok(ty);
            }
            if name.eq_ignore_ascii_case("IsMissing") {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "IsMissing expects exactly one argument",
                        Some(expr.span),
                    ));
                }
                let ExprKind::Variable(param_name) = &args[0].kind else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "IsMissing requires an optional parameter name",
                        Some(args[0].span),
                    ));
                };
                return match symbols.get(&key(param_name)) {
                    Some(VarType::Optional(_)) => Ok(TypeName::Boolean),
                    Some(_) => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "IsMissing is only valid for Optional parameters",
                        Some(args[0].span),
                    )),
                    None => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Variable '{}' is not declared", param_name),
                        Some(args[0].span),
                    )),
                };
            }
            if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
                if args.is_empty() || args.len() > 2 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("{} expects one array argument and optional dimension", name),
                        Some(expr.span),
                    ));
                }
                validate_array_expr(&args[0], symbols, types, signatures)?;
                if args.len() == 2 {
                    ensure_assignable(
                        &TypeName::Integer,
                        &validate_expr(&args[1], symbols, types, signatures)?,
                        args[1].span,
                    )?;
                }
                return Ok(TypeName::Integer);
            }
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                match var_type {
                    VarType::Array(element_type) => {
                        for arg in args {
                            ensure_assignable(
                                &TypeName::Integer,
                                &validate_expr(arg, symbols, types, signatures)?,
                                arg.span,
                            )?;
                        }
                        return Ok(element_type);
                    }
                    VarType::Scalar(TypeName::User(class_name))
                    | VarType::Optional(TypeName::User(class_name))
                    | VarType::Const(TypeName::User(class_name)) => {
                        if let Some(type_sig) = types.get(&class_name)
                            && type_sig.is_structure
                            && let Some(default_prop_name) = &type_sig.default_property
                        {
                            return validate_method_call(
                                &TypeName::User(class_name.clone()),
                                default_prop_name,
                                args,
                                true,
                                expr.span,
                                symbols,
                                types,
                                signatures,
                                None,
                            );
                        }
                        if let Some(default_prop_name) = types
                            .get_class(&class_name)
                            .and_then(|c| c.default_property.as_ref())
                        {
                            return validate_method_call(
                                &TypeName::User(class_name.clone()),
                                default_prop_name,
                                args,
                                true,
                                expr.span,
                                symbols,
                                types,
                                signatures,
                                None,
                            );
                        }
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!(
                                "Variable '{}' is not an array or a class with a default property",
                                name
                            ),
                            Some(expr.span),
                        ));
                    }
                    VarType::Scalar(TypeName::Variant)
                    | VarType::Optional(TypeName::Variant)
                    | VarType::Const(TypeName::Variant) => {
                        for arg in args {
                            validate_expr(arg, symbols, types, signatures)?;
                        }
                        return Ok(TypeName::Variant);
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Variable '{}' is not an array", name),
                            Some(expr.span),
                        ));
                    }
                }
            }
            if let Some(VarType::Scalar(TypeName::User(class_name))) = symbols.get("me").cloned()
                && let Some(class_sig) = types.get_class(&class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(name))
            {
                if field_sig.array.is_some() {
                    for arg in args {
                        ensure_assignable(
                            &TypeName::Integer,
                            &validate_expr(arg, symbols, types, signatures)?,
                            arg.span,
                        )?;
                    }
                    return Ok(field_sig.ty.clone());
                }
                if field_sig.ty.same_type(&TypeName::Variant) {
                    for arg in args {
                        validate_expr(arg, symbols, types, signatures)?;
                    }
                    return Ok(TypeName::Variant);
                }
            }

            let Some(function) = signatures.functions.get(&key(name)) else {
                if signatures.subs.contains_key(&key(name)) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Sub '{}' cannot be used as an expression", name),
                        Some(expr.span),
                    ));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Function '{}' is not defined", name),
                    Some(expr.span),
                ));
            };

            validate_arguments(
                "Function", function, args, symbols, types, signatures, expr.span,
            )?;
            Ok(function.return_type.clone().expect("function return type"))
        }
        ExprKind::Index { target, args } => {
            let _target_type = validate_expr(target, symbols, types, signatures)?;
            for arg in args {
                validate_expr(arg, symbols, types, signatures)?;
            }
            Ok(TypeName::Variant)
        }
        ExprKind::IIf {
            condition,
            true_expr,
            false_expr,
        } => {
            validate_expr(condition, symbols, types, signatures)?;
            let true_type = validate_expr(true_expr, symbols, types, signatures)?;
            let false_type = validate_expr(false_expr, symbols, types, signatures)?;
            if true_type.same_type(&false_type) {
                Ok(true_type)
            } else {
                Ok(TypeName::Variant)
            }
        }
        ExprKind::Binary { left, op, right } => {
            let left_type = validate_expr(left, symbols, types, signatures)?;
            let right_type = validate_expr(right, symbols, types, signatures)?;
            match op {
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Exponent
                | BinaryOp::Divide
                | BinaryOp::IntegerDivide
                | BinaryOp::Modulo => {
                    if is_numeric_type(&left_type) && is_numeric_type(&right_type) {
                        if left_type == TypeName::Double || right_type == TypeName::Double {
                            Ok(TypeName::Double)
                        } else if left_type == TypeName::Single || right_type == TypeName::Single {
                            Ok(TypeName::Single)
                        } else {
                            Ok(TypeName::Int64)
                        }
                    } else {
                        ensure_assignable(&TypeName::Int64, &left_type, left.span)?;
                        ensure_assignable(&TypeName::Int64, &right_type, right.span)?;
                        Ok(TypeName::Int64)
                    }
                }
                BinaryOp::Concat => Ok(TypeName::String),
                BinaryOp::LogicalAnd
                | BinaryOp::LogicalOr
                | BinaryOp::LogicalXor
                | BinaryOp::LogicalEqv
                | BinaryOp::LogicalImp => {
                    if left_type.same_type(&TypeName::Boolean)
                        && right_type.same_type(&TypeName::Boolean)
                    {
                        Ok(TypeName::Boolean)
                    } else if left_type.same_type(&TypeName::Integer)
                        && right_type.same_type(&TypeName::Integer)
                        || (is_enum_type(&left_type, types)
                            && right_type.same_type(&TypeName::Integer))
                        || (left_type.same_type(&TypeName::Integer)
                            && is_enum_type(&right_type, types))
                        || (is_enum_type(&left_type, types) && is_enum_type(&right_type, types))
                    {
                        Ok(TypeName::Integer)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            "Logical operators require Boolean or Integer operands",
                            Some(expr.span),
                        ))
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => Ok(TypeName::Boolean),
                BinaryOp::Like => {
                    ensure_assignable(&TypeName::String, &left_type, left.span)?;
                    ensure_assignable(&TypeName::String, &right_type, right.span)?;
                    Ok(TypeName::Boolean)
                }
                BinaryOp::Is => {
                    if is_object_reference_expr(left, &left_type, types)
                        && is_object_reference_expr(right, &right_type, types)
                    {
                        Ok(TypeName::Boolean)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "'Is' requires class object operands or Nothing",
                            Some(expr.span),
                        ))
                    }
                }
                BinaryOp::Less
                | BinaryOp::Greater
                | BinaryOp::LessEqual
                | BinaryOp::GreaterEqual => {
                    if (is_numeric_type(&left_type) && is_numeric_type(&right_type))
                        || (left_type.same_type(&TypeName::String)
                            && right_type.same_type(&TypeName::String))
                    {
                        Ok(TypeName::Boolean)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "Comparison requires matching numeric or String operands",
                            Some(expr.span),
                        ))
                    }
                }
            }
        }
        ExprKind::Unary { op, expr: inner } => match op {
            UnaryOp::Negate => {
                let ty = validate_expr(inner, symbols, types, signatures)?;
                if is_numeric_type(&ty) {
                    Ok(ty)
                } else {
                    ensure_assignable(&TypeName::Int64, &ty, inner.span)?;
                    Ok(TypeName::Int64)
                }
            }
            UnaryOp::LogicalNot => {
                ensure_assignable(
                    &TypeName::Boolean,
                    &validate_expr(inner, symbols, types, signatures)?,
                    inner.span,
                )?;
                Ok(TypeName::Boolean)
            }
        },
        ExprKind::AddressOf(_) => {
            // Wait, AddressOf returns a FuncPtr or LongPtr!
            Ok(TypeName::FuncPtr)
        }
    }
}

pub(super) fn validate_array_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    _signatures: &Signatures,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::Variable(name) => match symbols.get(&key(name)).cloned() {
            Some(VarType::Array(element_type)) => Ok(element_type),
            Some(VarType::Scalar(TypeName::Variant))
            | Some(VarType::Optional(TypeName::Variant))
            | Some(VarType::Const(TypeName::Variant)) => Ok(TypeName::Variant),
            Some(VarType::Scalar(_)) | Some(VarType::Optional(_)) | Some(VarType::Const(_)) => {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    format!("Variable '{}' is not an array", name),
                    Some(expr.span),
                ))
            }
            None => {
                if let Some(VarType::Scalar(TypeName::User(class_name))) =
                    symbols.get("me").cloned()
                    && let Some(class_sig) = types.get_class(&class_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.array.is_some() {
                        return Ok(field_sig.ty.clone());
                    }
                    if field_sig.ty.same_type(&TypeName::Variant) {
                        return Ok(TypeName::Variant);
                    }
                }
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(expr.span),
                ))
            }
        },
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::PARSE,
            "Expected array variable",
            Some(expr.span),
        )),
    }
}

fn validate_builtin_function(
    name: &str,
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
) -> Result<Option<TypeName>, Diagnostic> {
    // Handle VBA namespace fallback
    let effective_name = if let Some(stripped) = name.strip_prefix("VBA.") {
        stripped
    } else {
        name
    };

    if effective_name.eq_ignore_ascii_case("IsArray") {
        validate_arg_count(effective_name, args, 1, span)?;
        if validate_array_expr(&args[0], symbols, types, signatures).is_err() {
            validate_expr(&args[0], symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Boolean));
    }
    let boolean_one_arg = ["IsObject", "IsNull", "IsError", "IsEmpty"];
    if boolean_one_arg
        .iter()
        .any(|builtin| effective_name.eq_ignore_ascii_case(builtin))
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Boolean));
    }
    if effective_name.eq_ignore_ascii_case("IsMissing") {
        validate_arg_count(effective_name, args, 1, span)?;
        let arg = &args[0];
        if let ExprKind::Variable(name) = &arg.kind {
            if let Some(var_type) = symbols.get(&key(name))
                && !matches!(var_type, VarType::Optional(_))
            {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "IsMissing is only valid for Optional parameters",
                    Some(arg.span),
                ));
            }
        } else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "IsMissing is only valid for Optional parameters",
                Some(arg.span),
            ));
        }
        return Ok(Some(TypeName::Boolean));
    }
    if effective_name.eq_ignore_ascii_case("VarType")
        || effective_name.eq_ignore_ascii_case("Sgn")
        || effective_name.eq_ignore_ascii_case("Int")
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Integer));
    }
    if effective_name.eq_ignore_ascii_case("VarPtr")
        || effective_name.eq_ignore_ascii_case("StrPtr")
        || effective_name.eq_ignore_ascii_case("ObjPtr")
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Ptr));
    }
    if effective_name.eq_ignore_ascii_case("TypeName") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("CStr") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("CByte") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Byte));
    }
    if effective_name.eq_ignore_ascii_case("CInt") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Integer));
    }
    if effective_name.eq_ignore_ascii_case("CLng") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Long));
    }
    if effective_name.eq_ignore_ascii_case("CLngLng")
        || effective_name.eq_ignore_ascii_case("CInt64")
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Int64));
    }
    if effective_name.eq_ignore_ascii_case("CSng") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Single));
    }
    if effective_name.eq_ignore_ascii_case("CDbl") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Double));
    }
    if effective_name.eq_ignore_ascii_case("CDec") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Decimal));
    }
    if effective_name.eq_ignore_ascii_case("CCur") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Currency));
    }
    if effective_name.eq_ignore_ascii_case("CDate") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(&args[0], symbols, types, signatures)?;
        return Ok(Some(TypeName::Date));
    }
    if effective_name.eq_ignore_ascii_case("Array") {
        for arg in args {
            validate_expr(arg, symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Variant));
    }
    if effective_name.eq_ignore_ascii_case("Split") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Split expects 1 to 2 arguments",
                Some(span),
            ));
        }
        validate_expr(&args[0], symbols, types, signatures)?;
        if args.len() == 2 {
            validate_expr(&args[1], symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Variant));
    }
    if effective_name.eq_ignore_ascii_case("Join") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Join expects 1 to 2 arguments",
                Some(span),
            ));
        }
        validate_array_expr(&args[0], symbols, types, signatures)?;
        if args.len() == 2 {
            validate_expr(&args[1], symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("Filter") {
        if args.len() < 2 || args.len() > 4 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Filter expects 2 to 4 arguments",
                Some(span),
            ));
        }
        validate_array_expr(&args[0], symbols, types, signatures)?;
        validate_expr(&args[1], symbols, types, signatures)?;
        if args.len() >= 3 {
            validate_expr(&args[2], symbols, types, signatures)?;
        }
        if args.len() == 4 {
            validate_expr(&args[3], symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Variant));
    }
    if effective_name.eq_ignore_ascii_case("IIf") {
        validate_arg_count(effective_name, args, 3, span)?;
        ensure_assignable(
            &TypeName::Boolean,
            &validate_expr(&args[0], symbols, types, signatures)?,
            args[0].span,
        )?;
        validate_expr(&args[1], symbols, types, signatures)?;
        validate_expr(&args[2], symbols, types, signatures)?;
        return Ok(Some(TypeName::Variant));
    }
    if effective_name.eq_ignore_ascii_case("StrComp") {
        if args.len() < 2 || args.len() > 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "StrComp expects two strings and optional compare mode",
                Some(span),
            ));
        }
        validate_expr(&args[0], symbols, types, signatures)?;
        validate_expr(&args[1], symbols, types, signatures)?;
        if args.len() == 3 {
            ensure_assignable(
                &TypeName::Integer,
                &validate_expr(&args[2], symbols, types, signatures)?,
                args[2].span,
            )?;
        }
        return Ok(Some(TypeName::Integer));
    }
    if effective_name.eq_ignore_ascii_case("Randomize") {
        if args.len() > 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Randomize expects at most 1 argument",
                Some(span),
            ));
        }
        if !args.is_empty() {
            validate_expr(&args[0], symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Variant));
    }
    if effective_name.eq_ignore_ascii_case("Rnd") {
        if args.len() > 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Rnd expects at most 1 argument",
                Some(span),
            ));
        }
        if !args.is_empty() {
            validate_expr(&args[0], symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Double));
    }
    if effective_name.eq_ignore_ascii_case("CallByName") {
        if args.len() < 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CallByName expects at least 3 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures)?;
        }
        return Ok(Some(TypeName::Variant));
    }

    Ok(None)
}

fn validate_arg_count(
    name: &str,
    args: &[Expr],
    expected: usize,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects exactly {expected} argument(s)"),
            Some(span),
        ))
    }
}

pub(super) fn enum_member_value_type(name: &str, types: &TypeRegistry) -> Option<TypeName> {
    for enum_sig in types.enums.values() {
        if enum_sig.members.contains_key(&key(name)) {
            return Some(TypeName::Integer);
        }
    }
    None
}

pub(super) fn validate_arguments(
    kind: &str,
    callable: &CallableSig,
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    let has_param_array = callable
        .params
        .last()
        .is_some_and(|param| param.is_param_array);
    let mut assigned = vec![false; callable.params.len()];
    let mut positional_index = 0;
    let mut saw_named = false;

    for arg in args {
        if let ExprKind::NamedArg { name, expr: value } = &arg.kind {
            saw_named = true;
            let Some(index) = callable
                .params
                .iter()
                .position(|param| param.name.eq_ignore_ascii_case(name))
            else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!(
                        "{} '{}' has no parameter named '{}'",
                        kind, callable.name, name
                    ),
                    Some(arg.span),
                ));
            };
            if assigned[index] {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Argument '{}' is specified more than once", name),
                    Some(arg.span),
                ));
            }
            let param = &callable.params[index];
            if param.is_param_array {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "ParamArray arguments cannot be supplied by name",
                    Some(arg.span),
                ));
            }
            validate_argument_value(param, value, symbols, types, signatures)?;
            assigned[index] = true;
            continue;
        }
        if saw_named {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Positional arguments cannot appear after named arguments",
                Some(arg.span),
            ));
        }
        let Some(param) = callable
            .params
            .get(positional_index)
            .or_else(|| callable.params.last().filter(|param| param.is_param_array))
        else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!(
                    "{} '{}' expects {} argument(s), got {}",
                    kind,
                    callable.name,
                    callable.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        };
        validate_argument_value(param, arg, symbols, types, signatures)?;
        if !param.is_param_array {
            assigned[positional_index] = true;
            positional_index += 1;
        }
    }

    let missing_required = callable
        .params
        .iter()
        .enumerate()
        .any(|(index, param)| !assigned[index] && !param.is_optional && !param.is_param_array);
    if missing_required || (!has_param_array && args.len() > callable.params.len()) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!(
                "{} '{}' expects {} argument(s), got {}",
                kind,
                callable.name,
                callable.params.len(),
                args.len()
            ),
            Some(span),
        ));
    }

    Ok(())
}

fn validate_argument_value(
    param: &ParamSig,
    arg: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    if param.is_param_array {
        let arg_type = validate_expr(arg, symbols, types, signatures)?;
        ensure_assignable_expr(&TypeName::Variant, &arg_type, arg, types, arg.span)?;
        return Ok(());
    }
    match param.mode {
        PassingMode::ByVal => {
            let arg_type = validate_expr(arg, symbols, types, signatures)?;
            ensure_assignable_expr(&param.ty, &arg_type, arg, types, arg.span)
        }
        PassingMode::ByRef => {
            let ExprKind::Variable(name) = &arg.kind else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "ByRef argument must be a variable",
                    Some(arg.span),
                ));
            };
            let Some(arg_type) = symbols.get(&key(name)).cloned() else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Variable '{}' is not declared", name),
                    Some(arg.span),
                ));
            };
            let expected = VarType::Scalar(param.ty.clone());
            if !arg_type.same_var_type(&expected) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!(
                        "ByRef argument type {} must match parameter type {}",
                        arg_type.display_name(),
                        expected.display_name()
                    ),
                    Some(arg.span),
                ));
            }
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_method_call(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    if object_type.same_type(&TypeName::Variant) {
        return Ok(TypeName::Variant);
    }
    let TypeName::User(class_name) = object_type else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires a class instance",
            Some(span),
        ));
    };
    let Some(class_sig) = types.get_class(class_name) else {
        return validate_structure_method_call(
            object_type,
            method,
            args,
            as_expression,
            span,
            symbols,
            types,
            signatures,
            current_class,
        );
    };

    if as_expression {
        if let Some(method_sig) = class_sig.functions.get(&key(method)) {
            ensure_visible(
                method_sig.visibility,
                &class_sig.name,
                method,
                current_class,
                span,
            )?;
            validate_arguments(
                "Function", method_sig, args, symbols, types, signatures, span,
            )?;
            return Ok(method_sig.return_type.clone().expect("function return"));
        }
        if let Some(get) = class_sig
            .properties
            .get(&key(method))
            .and_then(|p| p.get.as_ref())
        {
            ensure_visible(get.visibility, &class_sig.name, method, current_class, span)?;
            let return_type = get.return_type.clone().expect("property return type");

            // Case 1: The property itself takes these arguments
            if get.params.len() == args.len() {
                // Try to validate arguments for the property Get
                let dummy_sig = CallableSig {
                    visibility: get.visibility,
                    name: method.to_string(),
                    _is_iterator: get.is_iterator,
                    params: get.params.clone(),
                    return_type: Some(return_type.clone()),
                };
                if validate_arguments(
                    "Property", &dummy_sig, args, symbols, types, signatures, span,
                )
                .is_ok()
                {
                    return Ok(return_type);
                }
            }

            // Case 2: The property returns an object that has a default property
            let default_call = match &return_type {
                TypeName::User(inner_class_name) => types
                    .get_class(inner_class_name)
                    .and_then(|c| c.default_property.as_ref())
                    .map(|name| (return_type.clone(), name.clone())),
                _ => None,
            };

            if let Some((inner_type, default_prop_name)) = default_call {
                return validate_method_call(
                    &inner_type,
                    &default_prop_name,
                    args,
                    true,
                    span,
                    symbols,
                    types,
                    signatures,
                    None,
                );
            }
        }
        if class_sig.subs.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Sub method '{}' cannot be used as an expression", method),
                Some(span),
            ));
        }
        if class_sig.events.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Event '{}' cannot be called directly", method),
                Some(span),
            ));
        }
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Class '{}' has no method or property '{}'",
                class_sig.name, method
            ),
            Some(span),
        ))
    } else {
        if let Some(method_sig) = class_sig.subs.get(&key(method)) {
            ensure_visible(
                method_sig.visibility,
                &class_sig.name,
                method,
                current_class,
                span,
            )?;
            validate_arguments("Sub", method_sig, args, symbols, types, signatures, span)?;
            return Ok(TypeName::Variant);
        }
        // Sub-style property call (e.g., obj.Prop = value or obj.Prop(idx) = value)
        // This is complex because MemberCall is usually for reads.
        // But some VBA code might use MemberCall as a statement for something that returns an object and then calls a default sub?
        // Actually MemberSubCall is used for subs.

        if class_sig.functions.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Function method '{}' cannot be called as a statement",
                    method
                ),
                Some(span),
            ));
        }
        if class_sig.events.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Event '{}' cannot be called directly", method),
                Some(span),
            ));
        }
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Class '{}' has no method '{}'", class_sig.name, method),
            Some(span),
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_structure_method_call(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    current_type: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    let TypeName::User(type_name) = object_type else {
        unreachable!();
    };
    let type_sig = types.get(type_name).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", type_name),
            Some(span),
        )
    })?;
    if !type_sig.is_structure {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires a class or Structure instance",
            Some(span),
        ));
    }
    if as_expression {
        if let Some(method_sig) = type_sig.functions.get(&key(method)) {
            ensure_visible(
                method_sig.visibility,
                &type_sig.name,
                method,
                current_type,
                span,
            )?;
            validate_arguments(
                "Function", method_sig, args, symbols, types, signatures, span,
            )?;
            return Ok(method_sig.return_type.clone().expect("function return"));
        }
        if let Some(get) = type_sig
            .properties
            .get(&key(method))
            .and_then(|p| p.get.as_ref())
        {
            ensure_visible(get.visibility, &type_sig.name, method, current_type, span)?;
            let return_type = get.return_type.clone().expect("property return type");
            let dummy_sig = CallableSig {
                visibility: get.visibility,
                name: method.to_string(),
                _is_iterator: get.is_iterator,
                params: get.params.clone(),
                return_type: Some(return_type.clone()),
            };
            validate_arguments(
                "Property", &dummy_sig, args, symbols, types, signatures, span,
            )?;
            return Ok(return_type);
        }
        if type_sig.subs.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Sub method '{}' cannot be used as an expression", method),
                Some(span),
            ));
        }
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Structure '{}' has no method or property '{}'",
                type_sig.name, method
            ),
            Some(span),
        ))
    } else {
        if method.eq_ignore_ascii_case("Constructor") || method.eq_ignore_ascii_case("Initialize") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Structure constructor cannot be called as a normal method",
                Some(span),
            ));
        }
        if let Some(method_sig) = type_sig.subs.get(&key(method)) {
            ensure_visible(
                method_sig.visibility,
                &type_sig.name,
                method,
                current_type,
                span,
            )?;
            validate_arguments("Sub", method_sig, args, symbols, types, signatures, span)?;
            return Ok(TypeName::Variant);
        }
        if type_sig.functions.contains_key(&key(method)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Function method '{}' cannot be called as a statement",
                    method
                ),
                Some(span),
            ));
        }
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Structure '{}' has no method '{}'", type_sig.name, method),
            Some(span),
        ))
    }
}

fn member_access_class(object: &Expr, object_type: &TypeName) -> Option<String> {
    if matches!(object.kind, ExprKind::Me)
        && let TypeName::User(name) = object_type
    {
        return Some(name.clone());
    }
    None
}

pub(super) fn member_read_type(
    object_type: &TypeName,
    member: &str,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    if object_type.same_type(&TypeName::Variant) {
        return Ok(TypeName::Variant);
    }
    let TypeName::User(type_name) = object_type else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    };

    if let Some(type_sig) = types.get(type_name) {
        if let Some(field) = type_sig.fields.get(&key(member)) {
            ensure_visible(
                field.visibility,
                &type_sig.name,
                member,
                current_class,
                span,
            )?;
            return Ok(field.ty.clone());
        }
        let Some(property_sig) = type_sig.properties.get(&key(member)) else {
            let message = if type_sig.is_structure {
                format!(
                    "Type '{}' has no field or property '{}'",
                    type_sig.name, member
                )
            } else {
                format!("Type '{}' has no field '{}'", type_sig.name, member)
            };
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                message,
                Some(span),
            ));
        };
        let get = property_sig.get.as_ref().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Property '{}' has no Get accessor", property_sig.name),
                Some(span),
            )
        })?;
        ensure_visible(get.visibility, &type_sig.name, member, current_class, span)?;
        return Ok(get.return_type.clone().expect("get return type"));
    }

    let class_sig = types.get_class(type_name).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", type_name),
            Some(span),
        )
    })?;
    if let Some(field_sig) = class_sig.fields.get(&key(member)) {
        ensure_visible(
            field_sig.visibility,
            &class_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(field_sig.ty.clone());
    }

    let property_sig = class_sig.properties.get(&key(member)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Class '{}' has no field or property '{}'",
                class_sig.name, member
            ),
            Some(span),
        )
    })?;
    let get = property_sig.get.as_ref().ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Property '{}' has no Get accessor", property_sig.name),
            Some(span),
        )
    })?;
    ensure_visible(get.visibility, &class_sig.name, member, current_class, span)?;
    Ok(get.return_type.clone().expect("get return type"))
}

fn member_assignment_type(
    object_type: &TypeName,
    member: &str,
    value_type: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    current_class: Option<&str>,
) -> Result<TypeName, Diagnostic> {
    if object_type.same_type(&TypeName::Variant) {
        return Ok(value_type.clone());
    }
    let TypeName::User(type_name) = object_type else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    };

    if let Some(type_sig) = types.get(type_name) {
        if let Some(field) = type_sig.fields.get(&key(member)) {
            ensure_visible(
                field.visibility,
                &type_sig.name,
                member,
                current_class,
                span,
            )?;
            return Ok(field.ty.clone());
        }
        let property_sig = type_sig.properties.get(&key(member)).ok_or_else(|| {
            let message = if type_sig.is_structure {
                format!(
                    "Type '{}' has no field or property '{}'",
                    type_sig.name, member
                )
            } else {
                format!("Type '{}' has no field '{}'", type_sig.name, member)
            };
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                message,
                Some(span),
            )
        })?;
        let accessor =
            if is_class_type(value_type, types) || value_type.same_type(&TypeName::Variant) {
                property_sig.set.as_ref().or(property_sig.let_.as_ref())
            } else {
                property_sig.let_.as_ref()
            }
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Property '{}' has no Let or Set accessor",
                        property_sig.name
                    ),
                    Some(span),
                )
            })?;
        ensure_visible(
            accessor.visibility,
            &type_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(accessor.params[0].ty.clone());
    }

    let class_sig = types.get_class(type_name).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", type_name),
            Some(span),
        )
    })?;
    if let Some(field_sig) = class_sig.fields.get(&key(member)) {
        ensure_visible(
            field_sig.visibility,
            &class_sig.name,
            member,
            current_class,
            span,
        )?;
        return Ok(field_sig.ty.clone());
    }

    let property_sig = class_sig.properties.get(&key(member)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Class '{}' has no field or property '{}'",
                class_sig.name, member
            ),
            Some(span),
        )
    })?;
    let accessor =
        if is_class_type(value_type, types) || value_type.same_type(&TypeName::Variant) {
            property_sig.set.as_ref().or(property_sig.let_.as_ref())
        } else {
            property_sig.let_.as_ref()
        }
        .ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Property '{}' has no Let or Set accessor",
                    property_sig.name
                ),
                Some(span),
            )
        })?;
    ensure_visible(
        accessor.visibility,
        &class_sig.name,
        member,
        current_class,
        span,
    )?;
    Ok(accessor.params[0].ty.clone())
}

fn ensure_visible(
    visibility: Visibility,
    owner_class: &str,
    member: &str,
    current_class: Option<&str>,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if visibility == Visibility::Public
        || current_class.is_some_and(|class_name| class_name.eq_ignore_ascii_case(owner_class))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
            format!("Member '{}' is Private in Class '{}'", member, owner_class),
            Some(span),
        )
        .with_primary_label("private member is not accessible here")
        .with_help("access this member from within the declaring class or make it Public"))
    }
}

pub(super) fn ensure_known_type(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    match ty {
        TypeName::String
        | TypeName::Byte
        | TypeName::Integer
        | TypeName::Long
        | TypeName::Int64
        | TypeName::UInt32
        | TypeName::UInt64
        | TypeName::Single
        | TypeName::Double
        | TypeName::Currency
        | TypeName::Decimal
        | TypeName::Boolean
        | TypeName::Date
        | TypeName::Variant
        | TypeName::Ptr
        | TypeName::FuncPtr => Ok(()),
        TypeName::User(name) => {
            if name.eq_ignore_ascii_case("Object") || types.contains(name) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Type '{}' is not defined", name),
                    Some(span),
                ))
            }
        }
    }
}

pub(super) fn ensure_assignable_expr(
    target: &TypeName,
    source: &TypeName,
    source_expr: &Expr,
    types: &TypeRegistry,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if matches!(source_expr.kind, ExprKind::Nothing) {
        return ensure_class_type(target, types, span, "Nothing requires a class object type");
    }
    if is_enum_type(target, types) && is_numeric_type(source) {
        return Ok(());
    }

    if let TypeName::User(class_name) = &source
        && let Some(class_sig) = types.get_class(class_name)
        && let Some(default_prop_name) = &class_sig.default_property
        && let Some(prop_sig) = class_sig.properties.get(&key(default_prop_name))
        && let Some(get) = &prop_sig.get
        && get.params.is_empty()
        && let Some(prop_type) = &get.return_type
        && ensure_assignable(target, prop_type, span).is_ok()
    {
        return Ok(());
    }

    ensure_assignable(target, source, span)
}

pub(super) fn ensure_class_type(
    ty: &TypeName,
    types: &TypeRegistry,
    span: crate::runtime::Span,
    message: &str,
) -> Result<(), Diagnostic> {
    if is_class_type(ty, types) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            message,
            Some(span),
        ))
    }
}

pub(super) fn is_object_reference_expr(expr: &Expr, ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(expr.kind, ExprKind::Nothing)
        || is_class_type(ty, types)
        || ty.same_type(&TypeName::Variant)
}

pub(super) fn is_class_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(ty, TypeName::User(name) if types.get_class(name).is_some() || name.eq_ignore_ascii_case("Object"))
}

pub(super) fn is_enum_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    matches!(ty, TypeName::User(name) if types.get_enum(name).is_some())
}

pub(super) fn ensure_case_comparable(
    subject: &TypeName,
    value: &TypeName,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if subject.same_type(&TypeName::Variant)
        || value.same_type(&TypeName::Variant)
        || subject.same_type(value)
        || (is_numeric_type(subject) && is_numeric_type(value))
        || (matches!(subject, TypeName::User(_)) && is_numeric_type(value))
        || (is_numeric_type(subject) && matches!(value, TypeName::User(_)))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::SELECT_CASE,
            "Case expression type must match Select Case expression type",
            Some(span),
        )
        .with_primary_label("case expression has an incompatible type"))
    }
}

pub(super) fn validate_case_item(
    item: &CaseItem,
    subject_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<(), Diagnostic> {
    match item {
        CaseItem::Value(value) => {
            let value_type = validate_expr(value, symbols, types, signatures)?;
            ensure_case_comparable(subject_type, &value_type, value.span)
        }
        CaseItem::Range { start, end } => {
            let start_type = validate_expr(start, symbols, types, signatures)?;
            let end_type = validate_expr(end, symbols, types, signatures)?;
            ensure_case_comparable(subject_type, &start_type, start.span)?;
            ensure_case_comparable(subject_type, &end_type, end.span)?;
            ensure_case_orderable(subject_type, start.span)
        }
        CaseItem::Compare { op, expr } => {
            let expr_type = validate_expr(expr, symbols, types, signatures)?;
            ensure_case_comparable(subject_type, &expr_type, expr.span)?;
            if matches!(
                op,
                CaseCompareOp::Less
                    | CaseCompareOp::Greater
                    | CaseCompareOp::LessEqual
                    | CaseCompareOp::GreaterEqual
            ) {
                ensure_case_orderable(subject_type, expr.span)?;
            }
            Ok(())
        }
    }
}

fn ensure_case_orderable(ty: &TypeName, span: crate::runtime::Span) -> Result<(), Diagnostic> {
    if is_numeric_type(ty) || ty.same_type(&TypeName::String) || ty.same_type(&TypeName::Variant) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::SELECT_CASE,
            "Case range or comparison requires numeric or String operands",
            Some(span),
        )
        .with_primary_label("range or comparison is not orderable"))
    }
}

pub(super) fn validate_exit(
    target: ExitTarget,
    span: crate::runtime::Span,
    context: &Context<'_>,
    loop_context: LoopContext,
) -> Result<(), Diagnostic> {
    match target {
        ExitTarget::Sub => match context {
            Context::Sub | Context::MethodSub { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Sub is only valid inside Sub",
                Some(span),
            )
            .with_primary_label("invalid Exit Sub")
            .with_help("use Exit Sub only inside a Sub body")),
        },
        ExitTarget::Function => match context {
            Context::Function { .. } | Context::MethodFunction { .. } => Ok(()),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(span),
            )
            .with_primary_label("invalid Exit Function")
            .with_help("use Exit Function only inside a Function body")),
        },
        ExitTarget::For => {
            if loop_context.for_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit For is only valid inside For",
                    Some(span),
                )
                .with_primary_label("invalid Exit For"))
            }
        }
        ExitTarget::While => {
            if loop_context.while_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit While is only valid inside While",
                    Some(span),
                )
                .with_primary_label("invalid Exit While"))
            }
        }
        ExitTarget::Do => {
            if loop_context.do_depth > 0 {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit Do is only valid inside Do",
                    Some(span),
                )
                .with_primary_label("invalid Exit Do"))
            }
        }
    }
}

pub(super) fn is_numeric_type(ty: &TypeName) -> bool {
    matches!(
        ty,
        TypeName::Byte
            | TypeName::Integer
            | TypeName::Long
            | TypeName::Int64
            | TypeName::UInt32
            | TypeName::UInt64
            | TypeName::Single
            | TypeName::Double
            | TypeName::Currency
            | TypeName::Decimal
            | TypeName::Date
    )
}

pub(super) fn ensure_assignable(
    target: &TypeName,
    source: &TypeName,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if target.same_type(&TypeName::Variant)
        || source.same_type(&TypeName::Variant)
        || target.same_type(source)
        || (is_numeric_type(target) && is_numeric_type(source))
        || (matches!(target, TypeName::Ptr | TypeName::FuncPtr) && is_numeric_type(source))
        || (is_numeric_type(target) && matches!(source, TypeName::Ptr | TypeName::FuncPtr))
        || (matches!(target, TypeName::Ptr | TypeName::FuncPtr)
            && matches!(source, TypeName::Ptr | TypeName::FuncPtr))
        || matches!(target, TypeName::User(name) if name.rsplit('.').next().is_some_and(|name| name.eq_ignore_ascii_case("Object")))
            && matches!(source, TypeName::User(_))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Cannot assign {} value to {} variable",
                source.display_name(),
                target.display_name()
            ),
            Some(span),
        )
        .with_primary_label(format!(
            "expected {}, found {}",
            target.display_name(),
            source.display_name()
        ))
        .with_help("change the variable type or assign a value with the expected type"))
    }
}

fn validate_err_raise_args(
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.is_empty() || args.len() > 5 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "Err.Raise expects 1 to 5 arguments",
            Some(span),
        ));
    }
    let expected = [
        TypeName::Integer,
        TypeName::String,
        TypeName::String,
        TypeName::String,
        TypeName::Integer,
    ];
    for (index, arg) in args.iter().enumerate() {
        let actual = validate_expr(arg, symbols, types, signatures)?;
        ensure_assignable(&expected[index], &actual, arg.span)?;
    }
    Ok(())
}
