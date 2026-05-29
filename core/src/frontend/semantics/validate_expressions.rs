use super::*;
use crate::runtime::Span;

#[derive(Clone, Copy)]
pub(super) struct ExprValidation<'a, 'ctx> {
    pub(super) symbols: &'a HashMap<String, VarType>,
    pub(super) types: &'a TypeRegistry,
    pub(super) signatures: &'a Signatures,
    pub(super) context: &'a Context<'ctx>,
    pub(super) option_explicit: bool,
}

impl<'a, 'ctx> ExprValidation<'a, 'ctx> {
    pub(super) fn new(
        symbols: &'a HashMap<String, VarType>,
        types: &'a TypeRegistry,
        signatures: &'a Signatures,
        context: &'a Context<'ctx>,
        option_explicit: bool,
    ) -> Self {
        Self {
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        }
    }
}

pub(super) fn validate_assignment_target(
    target: &AssignTarget,
    value_type: &TypeName,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<TypeName, Diagnostic> {
    match target {
        AssignTarget::Variable { name, span } => {
            let target_type = if let Some(target_type) = symbols.get(&key(name)).cloned() {
                target_type
            } else if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name)
                    && let Some(field_sig) = class_sig.fields.get(&key(name))
                {
                    if field_sig.is_shared || symbols.contains_key("me") {
                        VarType::Scalar(Visibility::Public, field_sig.ty.clone())
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Instance field '{}' cannot be accessed from a Shared method",
                                name
                            ),
                            Some(*span),
                        ));
                    }
                } else if let Some(type_sig) = types.get(owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    VarType::Scalar(Visibility::Public, field_sig.ty.clone())
                } else {
                    return Err(unknown_variable(name, *span, symbols));
                }
            } else {
                if !option_explicit {
                    return Ok(TypeName::Variant);
                }
                return Err(unknown_variable(name, *span, symbols));
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
                    if field_sig.is_shared || symbols.contains_key("me") {
                        if let Some(ref array_decl) = field_sig.array {
                            VarType::Array(
                                Visibility::Public,
                                field_sig.ty.clone(),
                                matches!(array_decl, ArrayDecl::Dynamic),
                            )
                        } else if field_sig.ty.same_type(&TypeName::Variant) {
                            VarType::Scalar(Visibility::Public, TypeName::Variant)
                        } else {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Variable '{}' is not an array", name),
                                Some(*span),
                            ));
                        }
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Instance array '{}' cannot be accessed from a Shared method",
                                name
                            ),
                            Some(*span),
                        ));
                    }
                } else if let Some(type_sig) = types.get(owner_name)
                    && let Some(field_sig) = type_sig.fields.get(&key(name))
                {
                    if let Some(ref array_decl) = field_sig.array {
                        VarType::Array(
                            Visibility::Public,
                            field_sig.ty.clone(),
                            matches!(array_decl, ArrayDecl::Dynamic),
                        )
                    } else if field_sig.ty.same_type(&TypeName::Variant) {
                        VarType::Scalar(Visibility::Public, TypeName::Variant)
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
                VarType::Array(_, ty, _) => ty,
                VarType::Scalar(_, TypeName::User(class_name))
                | VarType::Optional(_, TypeName::User(class_name))
                | VarType::Const(_, TypeName::User(class_name))
                    if class_name.eq_ignore_ascii_case("Object")
                        || class_name.eq_ignore_ascii_case("Collection") =>
                {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures, context, option_explicit)?;
                    }
                    return Ok(TypeName::Variant);
                }
                VarType::Scalar(_, TypeName::User(class_name))
                | VarType::Optional(_, TypeName::User(class_name))
                | VarType::Const(_, TypeName::User(class_name))
                    if types
                        .get_class(&class_name)
                        .and_then(|class| class.default_property.as_ref())
                        .is_some() =>
                {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures, context, option_explicit)?;
                    }
                    return Ok(value_type.clone());
                }
                VarType::Scalar(_, TypeName::Variant)
                | VarType::Optional(_, TypeName::Variant)
                | VarType::Const(_, TypeName::Variant) => {
                    for index in indices {
                        validate_expr(index, symbols, types, signatures, context, option_explicit)?;
                    }
                    return Ok(TypeName::Variant);
                }
                VarType::Module(alias) => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Module '{}' cannot be indexed", alias),
                        Some(*span),
                    ));
                }
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
                    &validate_expr(index, symbols, types, signatures, context, option_explicit)?,
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
            if let ExprKind::Variable(class_name) = &object.kind
                && !symbols.contains_key(&key(class_name))
                && let Some(class_sig) = types.get_class(class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(field))
                && field_sig.is_shared
            {
                return Ok(field_sig.ty.clone());
            }
            let object_type =
                validate_expr(object, symbols, types, signatures, context, option_explicit)?;
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
        AssignTarget::MemberArrayElement {
            object,
            field,
            indices,
            span,
        } => {
            let object_type =
                validate_expr(object, symbols, types, signatures, context, option_explicit)?;
            for index in indices {
                validate_expr(index, symbols, types, signatures, context, option_explicit)?;
            }
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

fn unknown_variable(name: &str, span: Span, symbols: &HashMap<String, VarType>) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
        format!("Variable '{}' is not declared", name),
        Some(span),
    )
    .with_primary_label("unknown variable")
    .with_help("declare the variable before using it")
    .with_name_suggestion(name, symbols.keys().map(String::as_str))
}

pub(super) fn validate_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::String(_) => Ok(TypeName::String),
        ExprKind::DateLiteral(_) => Ok(TypeName::Date),
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
        ExprKind::Long(_) => Ok(TypeName::Long),
        ExprKind::LongLong(_) => Ok(TypeName::Int64),
        ExprKind::Single(_) => Ok(TypeName::Single),
        ExprKind::Double(_) => Ok(TypeName::Double),
        ExprKind::Currency(_) => Ok(TypeName::Currency),
        ExprKind::Decimal(_) => Ok(TypeName::Decimal),
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
            let object_type =
                validate_expr(object, symbols, types, signatures, context, option_explicit)?;
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
            Some(VarType::Scalar(_, ty))
            | Some(VarType::Optional(_, ty))
            | Some(VarType::Const(_, ty)) => Ok(ty),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Me is only valid inside class methods",
                Some(expr.span),
            )),
        },
        ExprKind::MyBase | ExprKind::MyClass => match symbols.get("me").cloned() {
            Some(VarType::Scalar(_, ty))
            | Some(VarType::Optional(_, ty))
            | Some(VarType::Const(_, ty)) => Ok(ty),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "MyBase and MyClass are only valid inside class methods",
                Some(expr.span),
            )),
        },
        ExprKind::WithTarget => Ok(TypeName::Variant),
        ExprKind::New { class_name, args } => {
            if let TypeName::User(name) = class_name
                && name.eq_ignore_ascii_case("Collection")
            {
                return Ok(TypeName::User("Collection".to_string()));
            }
            ensure_known_type(class_name, types, expr.span)?;
            let (base_name, bindings) = generic_bindings_for_type(class_name, types);
            if let Some(type_sig) = types.get(&base_name) {
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
                    validate_arguments(
                        "Sub",
                        init,
                        args,
                        expr.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                } else if !args.is_empty() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Structure '{}' has no Sub New constructor", type_sig.name),
                        Some(expr.span),
                    ));
                }
                return Ok(types.canonical_type_name(class_name));
            }
            let class_sig = types.get_class(&base_name).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!(
                        "Class or Structure '{}' is not defined",
                        class_name.display_name()
                    ),
                    Some(expr.span),
                )
            })?;
            if let Some(init) = class_sig
                .subs
                .get("initialize")
                .or_else(|| class_sig.subs.get("class_initialize"))
            {
                validate_arguments(
                    "Sub",
                    init,
                    args,
                    expr.span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
            } else if !args.is_empty() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Class '{}' has no Initialize constructor", class_sig.name),
                    Some(expr.span),
                ));
            }
            let _ = bindings;
            Ok(types.canonical_type_name(class_name))
        }
        ExprKind::Variable(name) => {
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                match var_type {
                    VarType::Scalar(_, ty)
                    | VarType::Optional(_, ty)
                    | VarType::Const(_, ty)
                    | VarType::FunctionReturn(ty) => {
                        return Ok(ty);
                    }
                    VarType::Array(..) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            format!("Array variable '{}' cannot be used as a scalar", name),
                            Some(expr.span),
                        ));
                    }
                    VarType::Module(alias) => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            format!("Module '{}' cannot be used as an expression", alias),
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
            if name.eq_ignore_ascii_case("FreeFile") {
                return Ok(TypeName::Integer);
            }
            if name.eq_ignore_ascii_case("Timer") || name.eq_ignore_ascii_case("Rnd") {
                return Ok(TypeName::Double);
            }
            if name.eq_ignore_ascii_case("Now")
                || name.eq_ignore_ascii_case("Date")
                || name.eq_ignore_ascii_case("Time")
            {
                return Ok(TypeName::Date);
            }
            if name.eq_ignore_ascii_case("Console") {
                return Ok(TypeName::Variant);
            }
            if name.eq_ignore_ascii_case("VBA") {
                return Ok(TypeName::Variant);
            }
            if let Some(constant) = crate::runtime::vba::vba_constant(name) {
                return Ok(constant.type_name());
            }
            if let Some(function) = signatures.functions.get(&key(name)) {
                validate_arguments(
                    "Function",
                    function,
                    &[],
                    expr.span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;

                return Ok(function.return_type.clone().expect("function return type"));
            }
            if !option_explicit {
                return Ok(TypeName::Variant);
            }
            if let Some(owner_name) = context.current_class() {
                if let Some(class_sig) = types.get_class(owner_name) {
                    let member_key = key(name);
                    if let Some(field_sig) = class_sig.fields.get(&member_key)
                        && (field_sig.is_shared || symbols.contains_key("me"))
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
                    if let Some(func_sig) = class_sig.functions.get(&member_key)
                        && (func_sig.is_shared || symbols.contains_key("me"))
                    {
                        validate_arguments(
                            "Function",
                            func_sig,
                            &[],
                            expr.span,
                            ExprValidation::new(
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            ),
                        )?;
                        return Ok(func_sig.return_type.clone().expect("function return type"));
                    }
                    if let Some(prop_sig) = class_sig.properties.get(&member_key)
                        && (prop_sig.is_shared || symbols.contains_key("me"))
                        && let Some(get) = &prop_sig.get
                    {
                        let callable = CallableSig {
                            attributes: Vec::new(),
                            visibility: Visibility::Public,
                            name: prop_sig.name.clone(),
                            type_params: Vec::new(),
                            generic_constraints: Vec::new(),
                            is_shared: prop_sig.is_shared,
                            _is_iterator: get.is_iterator,
                            is_declare: false,
                            params: get.params.clone(),
                            return_type: get.return_type.clone(),
                        };
                        validate_arguments(
                            "Property",
                            &callable,
                            &[],
                            expr.span,
                            ExprValidation::new(
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            ),
                        )?;
                        return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                    }
                }
                if let Some(type_sig) = types.get(owner_name) {
                    let member_key = key(name);
                    if let Some(field_sig) = type_sig.fields.get(&member_key) {
                        if field_sig.array.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                format!("Array variable '{}' cannot be used as a scalar", name),
                                Some(expr.span),
                            ));
                        }
                        return Ok(field_sig.ty.clone());
                    }
                    if let Some(func_sig) = type_sig.functions.get(&member_key) {
                        validate_arguments(
                            "Function",
                            func_sig,
                            &[],
                            expr.span,
                            ExprValidation::new(
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            ),
                        )?;
                        return Ok(func_sig.return_type.clone().expect("function return type"));
                    }
                    if let Some(prop_sig) = type_sig.properties.get(&member_key)
                        && let Some(get) = &prop_sig.get
                    {
                        let callable = CallableSig {
                            attributes: Vec::new(),
                            visibility: Visibility::Public,
                            name: prop_sig.name.clone(),
                            type_params: Vec::new(),
                            generic_constraints: Vec::new(),
                            is_shared: prop_sig.is_shared,
                            _is_iterator: get.is_iterator,
                            is_declare: false,
                            params: get.params.clone(),
                            return_type: get.return_type.clone(),
                        };
                        validate_arguments(
                            "Property",
                            &callable,
                            &[],
                            expr.span,
                            ExprValidation::new(
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            ),
                        )?;
                        return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                    }
                }
            }
            if enum_member_value_type(name, types).is_some() {
                Ok(TypeName::Integer)
            } else if name.to_ascii_lowercase().starts_with("vb") {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("VBA runtime constant '{}' is unknown or unsupported", name),
                    Some(expr.span),
                ))
            } else if name.to_ascii_lowercase().starts_with("mso") {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!(
                        "Office/COM constant '{}' is not part of Valo core runtime constants",
                        name
                    ),
                    Some(expr.span),
                ))
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
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case("VBA")
            {
                if let Some(constant) = crate::runtime::vba::vba_constant(field) {
                    return Ok(constant.type_name());
                }
                if field.eq_ignore_ascii_case("Timer") || field.eq_ignore_ascii_case("Rnd") {
                    return Ok(TypeName::Double);
                }
                if field.eq_ignore_ascii_case("Now")
                    || field.eq_ignore_ascii_case("Date")
                    || field.eq_ignore_ascii_case("Time")
                {
                    return Ok(TypeName::Date);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Module 'VBA' has no member '{}'", field),
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
            if let ExprKind::Variable(class_name) = &object.kind
                && !symbols.contains_key(&key(class_name))
                && let Some(class_sig) = types.get_class(class_name)
            {
                if let Some(field_sig) = class_sig.fields.get(&key(field)) {
                    return Ok(field_sig.ty.clone());
                }
                if let Some(property_sig) = class_sig.properties.get(&key(field))
                    && let Some(get) = &property_sig.get
                {
                    return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Class '{}' has no Shared member '{}'",
                        class_sig.name, field
                    ),
                    Some(expr.span),
                ));
            }
            let object_type =
                validate_expr(object, symbols, types, signatures, context, option_explicit)?;
            if object_type.same_type(&TypeName::Variant) {
                return Ok(TypeName::Variant);
            }
            if let TypeName::User(name) = &object_type
                && name.eq_ignore_ascii_case("Object")
            {
                return Ok(TypeName::Variant);
            }
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
            type_args: _,
            args,
        } => {
            if let ExprKind::Variable(name) = &object.kind
                && name.eq_ignore_ascii_case("VBA")
            {
                if let Some(ty) = validate_builtin_function(
                    method,
                    args,
                    expr.span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )? {
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
                    validate_err_raise_args(
                        args,
                        symbols,
                        types,
                        signatures,
                        expr.span,
                        context,
                        option_explicit,
                    )?;
                    return Ok(TypeName::Variant);
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Err only supports Clear() and Raise()",
                    Some(expr.span),
                ));
            }
            if let ExprKind::Variable(class_name) = &object.kind
                && !symbols.contains_key(&key(class_name))
                && let Some(class_sig) = types.get_class(class_name)
            {
                if let Some(function) = class_sig.functions.get(&key(method)) {
                    validate_arguments(
                        "Function",
                        function,
                        args,
                        expr.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    return Ok(function.return_type.clone().unwrap_or(TypeName::Variant));
                }
                if let Some(property_sig) = class_sig.properties.get(&key(method))
                    && let Some(get) = &property_sig.get
                {
                    let callable = CallableSig {
                        attributes: Vec::new(),
                        visibility: Visibility::Public,
                        name: property_sig.name.clone(),
                        type_params: Vec::new(),
                        generic_constraints: Vec::new(),
                        is_shared: property_sig.is_shared,
                        _is_iterator: get.is_iterator,
                        is_declare: false,
                        params: get.params.clone(),
                        return_type: get.return_type.clone(),
                    };
                    validate_arguments(
                        "Property",
                        &callable,
                        args,
                        expr.span,
                        ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    )?;
                    return Ok(get.return_type.clone().unwrap_or(TypeName::Variant));
                }
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Class '{}' has no Shared method '{}'",
                        class_sig.name, method
                    ),
                    Some(expr.span),
                ));
            }
            let object_type =
                validate_expr(object, symbols, types, signatures, context, option_explicit)?;
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
                context,
                option_explicit,
            )
        }
        ExprKind::Call {
            name,
            type_args,
            args,
        } => {
            if let Some(ty) = validate_builtin_function(
                name,
                args,
                expr.span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
            )? {
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
                    Some(VarType::Optional(Visibility::Public, _)) => Ok(TypeName::Boolean),
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
                validate_array_expr(
                    &args[0],
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?;
                if args.len() == 2 {
                    ensure_assignable(
                        &TypeName::Integer,
                        &validate_expr(
                            &args[1],
                            symbols,
                            types,
                            signatures,
                            context,
                            option_explicit,
                        )?,
                        args[1].span,
                    )?;
                }
                return Ok(TypeName::Integer);
            }
            if let Some(var_type) = symbols.get(&key(name)).cloned() {
                match var_type {
                    VarType::Array(Visibility::Public, element_type, _) => {
                        for arg in args {
                            ensure_assignable(
                                &TypeName::Integer,
                                &validate_expr(
                                    arg,
                                    symbols,
                                    types,
                                    signatures,
                                    context,
                                    option_explicit,
                                )?,
                                arg.span,
                            )?;
                        }
                        return Ok(element_type);
                    }
                    VarType::Scalar(_, TypeName::User(class_name))
                    | VarType::Optional(_, TypeName::User(class_name))
                    | VarType::Const(_, TypeName::User(class_name)) => {
                        if class_name.eq_ignore_ascii_case("Object") {
                            for arg in args {
                                validate_expr(
                                    arg,
                                    symbols,
                                    types,
                                    signatures,
                                    context,
                                    option_explicit,
                                )?;
                            }
                            return Ok(TypeName::Variant);
                        }
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
                                context,
                                option_explicit,
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
                                context,
                                option_explicit,
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
                    VarType::Scalar(_, TypeName::Variant)
                    | VarType::Optional(_, TypeName::Variant)
                    | VarType::Const(_, TypeName::Variant) => {
                        for arg in args {
                            validate_expr(
                                arg,
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            )?;
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
            if let Some(VarType::Scalar(Visibility::Public, TypeName::User(class_name))) =
                symbols.get("me").cloned()
                && let Some(class_sig) = types.get_class(&class_name)
                && let Some(field_sig) = class_sig.fields.get(&key(name))
            {
                if field_sig.array.is_some() {
                    for arg in args {
                        ensure_assignable(
                            &TypeName::Integer,
                            &validate_expr(
                                arg,
                                symbols,
                                types,
                                signatures,
                                context,
                                option_explicit,
                            )?,
                            arg.span,
                        )?;
                    }
                    return Ok(field_sig.ty.clone());
                }
                if field_sig.ty.same_type(&TypeName::Variant) {
                    for arg in args {
                        validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
                    }
                    return Ok(TypeName::Variant);
                }
            }

            let mut function = signatures.functions.get(&key(name)).cloned();
            if function.is_none()
                && let Some(owner_name) = context.current_class()
                && let Some(class_sig) = types.get_class(owner_name)
            {
                let member_key = key(name);
                if let Some(func_sig) = class_sig.functions.get(&member_key) {
                    if func_sig.is_shared || symbols.contains_key("me") {
                        function = Some(func_sig.clone());
                    }
                } else if let Some(prop_sig) = class_sig.properties.get(&member_key)
                    && let Some(get) = &prop_sig.get
                    && (prop_sig.is_shared || symbols.contains_key("me"))
                {
                    function = Some(CallableSig {
                        attributes: Vec::new(),
                        visibility: Visibility::Public,
                        name: prop_sig.name.clone(),
                        type_params: Vec::new(),
                        generic_constraints: Vec::new(),
                        is_shared: prop_sig.is_shared,
                        _is_iterator: get.is_iterator,
                        is_declare: false,
                        params: get.params.clone(),
                        return_type: get.return_type.clone(),
                    });
                }
            }

            let Some(function) = function else {
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

            let inferred_type_args;
            let type_args = if type_args.is_empty() && !function.type_params.is_empty() {
                inferred_type_args = infer_callable_type_args(
                    &function,
                    args,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                    expr.span,
                )?;
                &inferred_type_args
            } else {
                type_args
            };
            let function = instantiate_callable(&function, type_args, expr.span, types)?;
            validate_arguments(
                "Function",
                &function,
                args,
                expr.span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
            )?;
            Ok(function.return_type.clone().expect("function return type"))
        }
        ExprKind::Index { target, args } => {
            let _target_type =
                validate_expr(target, symbols, types, signatures, context, option_explicit)?;
            for arg in args {
                validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
            }
            Ok(TypeName::Variant)
        }
        ExprKind::IIf {
            condition,
            true_expr,
            false_expr,
        } => {
            validate_expr(
                condition,
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
            let true_type = validate_expr(
                true_expr,
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
            let false_type = validate_expr(
                false_expr,
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
            if true_type.same_type(&false_type) {
                Ok(true_type)
            } else {
                Ok(TypeName::Variant)
            }
        }
        ExprKind::Binary { left, op, right } => {
            let left_type =
                validate_expr(left, symbols, types, signatures, context, option_explicit)?;
            let right_type =
                validate_expr(right, symbols, types, signatures, context, option_explicit)?;

            let operator_kind = match op {
                BinaryOp::Add => Some(crate::OperatorKind::Add),
                BinaryOp::Subtract => Some(crate::OperatorKind::Subtract),
                BinaryOp::Multiply => Some(crate::OperatorKind::Multiply),
                BinaryOp::Divide => Some(crate::OperatorKind::Divide),
                BinaryOp::IntegerDivide => Some(crate::OperatorKind::IntegerDivide),
                BinaryOp::Exponent => Some(crate::OperatorKind::Exponent),
                BinaryOp::Modulo => Some(crate::OperatorKind::Modulo),
                BinaryOp::LogicalAnd => Some(crate::OperatorKind::And),
                BinaryOp::LogicalOr => Some(crate::OperatorKind::Or),
                BinaryOp::LogicalXor => Some(crate::OperatorKind::Xor),
                BinaryOp::Equal => Some(crate::OperatorKind::Equal),
                BinaryOp::NotEqual => Some(crate::OperatorKind::NotEqual),
                BinaryOp::Less => Some(crate::OperatorKind::Less),
                BinaryOp::Greater => Some(crate::OperatorKind::Greater),
                BinaryOp::LessEqual => Some(crate::OperatorKind::LessEqual),
                BinaryOp::GreaterEqual => Some(crate::OperatorKind::GreaterEqual),
                BinaryOp::Like => Some(crate::OperatorKind::Like),
                BinaryOp::Concat => Some(crate::OperatorKind::Concatenate),
                _ => None,
            };

            if let Some(kind) = operator_kind
                && let Some(res_ty) =
                    find_overloaded_binary_operator(&left_type, kind, &right_type, types)
            {
                return Ok(res_ty);
            }

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
                    if (left_type.same_type(&TypeName::Boolean)
                        || left_type.same_type(&TypeName::Variant))
                        && (right_type.same_type(&TypeName::Boolean)
                            || right_type.same_type(&TypeName::Variant))
                    {
                        Ok(TypeName::Boolean)
                    } else if (left_type.same_type(&TypeName::Integer)
                        || left_type.same_type(&TypeName::Variant))
                        && (right_type.same_type(&TypeName::Integer)
                            || right_type.same_type(&TypeName::Variant))
                        || (is_enum_type(&left_type, types)
                            && (right_type.same_type(&TypeName::Integer)
                                || right_type.same_type(&TypeName::Variant)))
                        || ((left_type.same_type(&TypeName::Integer)
                            || left_type.same_type(&TypeName::Variant))
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
        ExprKind::Unary { op, expr: inner } => {
            let ty = validate_expr(inner, symbols, types, signatures, context, option_explicit)?;
            let operator_kind = match op {
                UnaryOp::Positive => Some(crate::OperatorKind::UnaryPlus),
                UnaryOp::Negate => Some(crate::OperatorKind::UnaryMinus),
                UnaryOp::LogicalNot => Some(crate::OperatorKind::Not),
            };
            if let Some(kind) = operator_kind
                && let Some(res_ty) = find_overloaded_unary_operator(kind, &ty, types)
            {
                return Ok(res_ty);
            }
            match op {
                UnaryOp::Positive | UnaryOp::Negate => {
                    if is_numeric_type(&ty) {
                        Ok(ty)
                    } else {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            format!(
                                "Unary '{}' requires a numeric expression",
                                if matches!(op, UnaryOp::Negate) {
                                    "-"
                                } else {
                                    "+"
                                }
                            ),
                            Some(inner.span),
                        ))
                    }
                }
                UnaryOp::LogicalNot => {
                    ensure_assignable(&TypeName::Boolean, &ty, inner.span)?;
                    Ok(TypeName::Boolean)
                }
            }
        }
        ExprKind::AddressOf(_) => {
            // Wait, AddressOf returns a FuncPtr or LongPtr!
            Ok(TypeName::FuncPtr)
        }
        ExprKind::PassingModeOverride { expr, .. } => {
            validate_expr(expr, symbols, types, signatures, context, option_explicit)
        }
    }
}

pub(super) fn validate_array_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    _signatures: &Signatures,
    _context: &Context<'_>,
    option_explicit: bool,
) -> Result<TypeName, Diagnostic> {
    match &expr.kind {
        ExprKind::Variable(name) => match symbols.get(&key(name)).cloned() {
            Some(VarType::Array(_, element_type, _)) => Ok(element_type),
            Some(VarType::Scalar(_, TypeName::Variant))
            | Some(VarType::Optional(_, TypeName::Variant))
            | Some(VarType::Const(_, TypeName::Variant)) => Ok(TypeName::Variant),
            Some(VarType::Scalar(_, TypeName::User(class_name)))
            | Some(VarType::Optional(_, TypeName::User(class_name)))
            | Some(VarType::Const(_, TypeName::User(class_name)))
                if class_name.eq_ignore_ascii_case("Object") =>
            {
                Ok(TypeName::Variant)
            }
            Some(VarType::Scalar(_, _ty))
            | Some(VarType::Optional(_, _ty))
            | Some(VarType::Const(_, _ty)) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                format!("Variable '{}' is not an array", name),
                Some(expr.span),
            )),
            Some(VarType::Module(alias)) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("Module '{}' cannot be used as an array", alias),
                Some(expr.span),
            )),
            Some(VarType::FunctionReturn(_)) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                format!("Variable '{}' is not an array", name),
                Some(expr.span),
            )),
            None => {
                if let Some(VarType::Scalar(_, TypeName::User(class_name))) =
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
                if !option_explicit {
                    return Ok(TypeName::Variant);
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
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<Option<TypeName>, Diagnostic> {
    let symbols = validation.symbols;
    let types = validation.types;
    let signatures = validation.signatures;
    let context = validation.context;
    let option_explicit = validation.option_explicit;

    // Handle VBA namespace fallback
    let effective_name = if let Some(stripped) = name.strip_prefix("VBA.") {
        stripped
    } else {
        name
    };

    if effective_name.eq_ignore_ascii_case("IsArray") {
        validate_arg_count(effective_name, args, 1, span)?;
        if validate_array_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )
        .is_err()
        {
            validate_expr(
                &args[0],
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.option_explicit,
            )?;
        }
        return Ok(Some(TypeName::Boolean));
    }
    let boolean_one_arg = [
        "IsObject",
        "IsNull",
        "IsError",
        "IsEmpty",
        "IsNumeric",
        "IsDate",
    ];
    if boolean_one_arg
        .iter()
        .any(|builtin| effective_name.eq_ignore_ascii_case(builtin))
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Boolean));
    }
    if effective_name.eq_ignore_ascii_case("TypeName") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("Split") {
        if args.is_empty() || args.len() > 4 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Split expects 1 to 4 arguments",
                Some(span),
            ));
        }
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
        return Ok(Some(TypeName::Array(Box::new(TypeName::String))));
    }
    if effective_name.eq_ignore_ascii_case("Join") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Join expects 1 to 2 arguments",
                Some(span),
            ));
        }
        validate_array_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        if args.len() == 2 {
            validate_expr(
                &args[1],
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.option_explicit,
            )?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("IsMissing") {
        validate_arg_count(effective_name, args, 1, span)?;
        let arg = &args[0];
        if let ExprKind::Variable(name) = &arg.kind {
            if let Some(var_type) = symbols.get(&key(name))
                && !matches!(var_type, VarType::Optional(Visibility::Public, _))
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
        || effective_name.eq_ignore_ascii_case("Len")
        || effective_name.eq_ignore_ascii_case("LenB")
        || effective_name.eq_ignore_ascii_case("Asc")
        || effective_name.eq_ignore_ascii_case("AscW")
        || effective_name.eq_ignore_ascii_case("FreeFile")
        || effective_name.eq_ignore_ascii_case("LOF")
        || effective_name.eq_ignore_ascii_case("Seek")
        || effective_name.eq_ignore_ascii_case("FileLen")
        || effective_name.eq_ignore_ascii_case("Year")
        || effective_name.eq_ignore_ascii_case("Month")
        || effective_name.eq_ignore_ascii_case("Day")
        || effective_name.eq_ignore_ascii_case("Hour")
        || effective_name.eq_ignore_ascii_case("Minute")
        || effective_name.eq_ignore_ascii_case("Second")
    {
        let expected = if effective_name.eq_ignore_ascii_case("FreeFile") {
            0
        } else {
            1
        };
        validate_arg_count(effective_name, args, expected, span)?;
        if expected == 1 {
            validate_expr(
                &args[0],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
        }
        return Ok(Some(TypeName::Integer));
    }
    if effective_name.eq_ignore_ascii_case("Timer") {
        validate_arg_count(effective_name, args, 0, span)?;
        return Ok(Some(TypeName::Double));
    }
    if effective_name.eq_ignore_ascii_case("Now")
        || effective_name.eq_ignore_ascii_case("Date")
        || effective_name.eq_ignore_ascii_case("Time")
    {
        validate_arg_count(effective_name, args, 0, span)?;
        return Ok(Some(TypeName::Date));
    }
    if effective_name.eq_ignore_ascii_case("DateSerial")
        || effective_name.eq_ignore_ascii_case("TimeSerial")
    {
        validate_arg_count(effective_name, args, 3, span)?;
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::Date));
    }
    if effective_name.eq_ignore_ascii_case("DateValue")
        || effective_name.eq_ignore_ascii_case("TimeValue")
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Date));
    }
    if effective_name.eq_ignore_ascii_case("Weekday") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Weekday expects 1 to 2 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::Integer));
    }
    if effective_name.eq_ignore_ascii_case("MonthName") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "MonthName expects 1 to 2 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("WeekdayName") {
        if args.is_empty() || args.len() > 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "WeekdayName expects 1 to 3 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("FileDateTime") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Date));
    }
    if effective_name.eq_ignore_ascii_case("EOF") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Boolean));
    }
    if effective_name.eq_ignore_ascii_case("Dir") {
        if args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Dir expects 0 to 2 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("CurDir") {
        if args.len() > 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CurDir expects 0 to 1 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("VarPtr")
        || effective_name.eq_ignore_ascii_case("StrPtr")
        || effective_name.eq_ignore_ascii_case("ObjPtr")
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Ptr));
    }
    if effective_name.eq_ignore_ascii_case("TypeName") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("CreateObject") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CreateObject expects 1 to 2 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::User("Object".to_string())));
    }
    if effective_name.eq_ignore_ascii_case("CStr") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("CByte") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Byte));
    }
    if effective_name.eq_ignore_ascii_case("CInt") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Integer));
    }
    if effective_name.eq_ignore_ascii_case("CLng") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Long));
    }
    if effective_name.eq_ignore_ascii_case("CLngLng")
        || effective_name.eq_ignore_ascii_case("CInt64")
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Int64));
    }
    if effective_name.eq_ignore_ascii_case("CSng") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Single));
    }
    if effective_name.eq_ignore_ascii_case("CDbl") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Double));
    }
    if effective_name.eq_ignore_ascii_case("CDec") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Decimal));
    }
    if effective_name.eq_ignore_ascii_case("CCur") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Currency));
    }
    if effective_name.eq_ignore_ascii_case("CDate") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Date));
    }
    if effective_name.eq_ignore_ascii_case("CBool") {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        return Ok(Some(TypeName::Boolean));
    }
    if effective_name.eq_ignore_ascii_case("Array") {
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
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
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        if args.len() == 2 {
            validate_expr(
                &args[1],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
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
        validate_array_expr(
            &args[0],
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        )?;
        if args.len() == 2 {
            validate_expr(
                &args[1],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
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
        validate_array_expr(
            &args[0],
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        )?;
        validate_expr(
            &args[1],
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        )?;
        if args.len() >= 3 {
            validate_expr(
                &args[2],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
        }
        if args.len() == 4 {
            validate_expr(
                &args[3],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
        }
        return Ok(Some(TypeName::Variant));
    }
    if effective_name.eq_ignore_ascii_case("IIf") {
        validate_arg_count(effective_name, args, 3, span)?;
        ensure_assignable(
            &TypeName::Boolean,
            &validate_expr(
                &args[0],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?,
            args[0].span,
        )?;
        validate_expr(
            &args[1],
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        )?;
        validate_expr(
            &args[2],
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        )?;
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
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        validate_expr(
            &args[1],
            symbols,
            types,
            signatures,
            context,
            option_explicit,
        )?;
        if args.len() == 3 {
            ensure_assignable(
                &TypeName::Integer,
                &validate_expr(
                    &args[2],
                    symbols,
                    types,
                    signatures,
                    context,
                    option_explicit,
                )?,
                args[2].span,
            )?;
        }
        return Ok(Some(TypeName::Integer));
    }
    let string_one_arg = [
        "Trim", "LTrim", "RTrim", "UCase", "LCase", "Chr", "ChrW", "Str", "Hex", "Oct", "Val",
    ];
    if string_one_arg
        .iter()
        .any(|builtin| effective_name.eq_ignore_ascii_case(builtin))
    {
        validate_arg_count(effective_name, args, 1, span)?;
        validate_expr(
            &args[0],
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        let return_type = if effective_name.eq_ignore_ascii_case("Val") {
            TypeName::Double
        } else {
            TypeName::String
        };
        return Ok(Some(return_type));
    }
    let string_two_arg = ["Left", "Right", "Space", "String"];
    if string_two_arg
        .iter()
        .any(|builtin| effective_name.eq_ignore_ascii_case(builtin))
    {
        validate_arg_count(
            effective_name,
            args,
            if effective_name.eq_ignore_ascii_case("Space") {
                1
            } else {
                2
            },
            span,
        )?;
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("Mid") {
        if args.len() < 2 || args.len() > 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Mid expects 2 to 3 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("Replace") {
        if args.len() < 3 || args.len() > 6 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Replace expects 3 to 6 arguments",
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(Some(TypeName::String));
    }
    if effective_name.eq_ignore_ascii_case("InStr")
        || effective_name.eq_ignore_ascii_case("InStrRev")
    {
        if args.len() < 2 || args.len() > 4 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("{effective_name} expects 2 to 4 arguments"),
                Some(span),
            ));
        }
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
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
            validate_expr(
                &args[0],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
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
            validate_expr(
                &args[0],
                symbols,
                types,
                signatures,
                context,
                option_explicit,
            )?;
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
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
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
        let code = if args.len() < expected {
            crate::runtime::DiagnosticCode::ARGUMENT_NOT_OPTIONAL
        } else {
            crate::runtime::DiagnosticCode::GENERIC
        };
        Err(Diagnostic::new(
            code,
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
    span: Span,
    validation: ExprValidation<'_, '_>,
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
            validate_argument_value(param, value, validation)?;
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
        validate_argument_value(param, arg, validation)?;
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
    validation: ExprValidation<'_, '_>,
) -> Result<(), Diagnostic> {
    if param.is_param_array {
        let arg_type = validate_expr(
            arg,
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?;
        ensure_assignable_expr(
            &TypeName::Variant,
            &arg_type,
            arg,
            validation.types,
            arg.span,
        )?;
        return Ok(());
    }
    match param.mode {
        PassingMode::ByVal => {
            let arg_type = validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.option_explicit,
            )?;
            ensure_assignable_expr(&param.ty, &arg_type, arg, validation.types, arg.span)
        }
        PassingMode::ByRef => {
            let arg_type = validate_expr(
                arg,
                validation.symbols,
                validation.types,
                validation.signatures,
                validation.context,
                validation.option_explicit,
            )?;
            ensure_assignable_expr(&param.ty, &arg_type, arg, validation.types, arg.span)
        }
    }
}

fn instantiate_callable(
    callable: &CallableSig,
    type_args: &[TypeName],
    span: Span,
    types: &TypeRegistry,
) -> Result<CallableSig, Diagnostic> {
    if callable.type_params.is_empty() {
        if !type_args.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                format!("'{}' is not generic", callable.name),
                Some(span),
            ));
        }
        return Ok(callable.clone());
    }
    if callable.type_params.len() != type_args.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Type parameter count mismatch for {}. Expected {}, received {}",
                callable.name,
                callable.type_params.len(),
                type_args.len()
            ),
            Some(span),
        ));
    }
    validate_generic_constraints(
        &callable.name,
        &callable.type_params,
        &callable.generic_constraints,
        type_args,
        types,
        span,
    )?;
    let bindings = callable
        .type_params
        .iter()
        .cloned()
        .zip(type_args.iter().cloned())
        .collect::<Vec<_>>();
    let mut instantiated = callable.clone();
    instantiated.type_params.clear();
    for param in &mut instantiated.params {
        param.ty = param.ty.substitute_generics(&bindings);
    }
    instantiated.return_type = instantiated
        .return_type
        .map(|ty| ty.substitute_generics(&bindings));
    Ok(instantiated)
}

fn infer_callable_type_args(
    callable: &CallableSig,
    args: &[Expr],
    validation: ExprValidation<'_, '_>,
    span: Span,
) -> Result<Vec<TypeName>, Diagnostic> {
    let mut inferred: Vec<Option<TypeName>> = vec![None; callable.type_params.len()];
    let mut positional_index = 0;
    for arg in args {
        let (param, arg_expr) = if let ExprKind::NamedArg { name, expr } = &arg.kind {
            let Some(param) = callable
                .params
                .iter()
                .find(|param| param.name.eq_ignore_ascii_case(name))
            else {
                continue;
            };
            (param, expr.as_ref())
        } else {
            let Some(param) = callable.params.get(positional_index) else {
                continue;
            };
            positional_index += 1;
            (param, arg)
        };
        let Some(arg_type) = infer_expr_type_for_generic(
            arg_expr,
            validation.symbols,
            validation.types,
            validation.signatures,
            validation.context,
            validation.option_explicit,
        )?
        else {
            continue;
        };
        collect_generic_type_inferences(
            &param.ty,
            &arg_type,
            &callable.type_params,
            &mut inferred,
            arg.span,
        )?;
    }

    inferred
        .into_iter()
        .enumerate()
        .map(|(index, ty)| {
            ty.ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Cannot infer type argument '{}' for '{}'",
                        callable.type_params[index], callable.name
                    ),
                    Some(span),
                )
                .with_help("specify the type argument explicitly with '(Of ...)'")
            })
        })
        .collect()
}

fn infer_expr_type_for_generic(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<Option<TypeName>, Diagnostic> {
    match &expr.kind {
        ExprKind::String(_) => Ok(Some(TypeName::String)),
        ExprKind::DateLiteral(_) => Ok(Some(TypeName::Date)),
        ExprKind::Integer(value) => {
            let ty = if *value >= i16::MIN as i64 && *value <= i16::MAX as i64 {
                TypeName::Integer
            } else if *value >= i32::MIN as i64 && *value <= i32::MAX as i64 {
                TypeName::Long
            } else {
                TypeName::Int64
            };
            Ok(Some(ty))
        }
        ExprKind::Long(_) => Ok(Some(TypeName::Long)),
        ExprKind::LongLong(_) => Ok(Some(TypeName::Int64)),
        ExprKind::Single(_) => Ok(Some(TypeName::Single)),
        ExprKind::Double(_) => Ok(Some(TypeName::Double)),
        ExprKind::Currency(_) => Ok(Some(TypeName::Currency)),
        ExprKind::Decimal(_) => Ok(Some(TypeName::Decimal)),
        ExprKind::Boolean(_) => Ok(Some(TypeName::Boolean)),
        ExprKind::Variable(name) => Ok(symbols.get(&key(name)).and_then(VarType::scalar_type)),
        ExprKind::New { class_name, .. } => Ok(Some(types.canonical_type_name(class_name))),
        ExprKind::NamedArg { expr, .. } | ExprKind::PassingModeOverride { expr, .. } => {
            infer_expr_type_for_generic(expr, symbols, types, signatures, context, option_explicit)
        }
        ExprKind::Nothing | ExprKind::Empty | ExprKind::Null | ExprKind::Missing => {
            Ok(Some(TypeName::Variant))
        }
        ExprKind::AddressOf(_) => Ok(Some(TypeName::FuncPtr)),
        ExprKind::Me | ExprKind::MyBase | ExprKind::MyClass => {
            validate_expr(expr, symbols, types, signatures, context, option_explicit).map(Some)
        }
        _ => Ok(None),
    }
}

fn collect_generic_type_inferences(
    param_type: &TypeName,
    arg_type: &TypeName,
    type_params: &[String],
    inferred: &mut [Option<TypeName>],
    span: Span,
) -> Result<(), Diagnostic> {
    match param_type {
        TypeName::User(name) => {
            if let Some(index) = type_params
                .iter()
                .position(|param| param.eq_ignore_ascii_case(name))
            {
                merge_inferred_type(&mut inferred[index], arg_type.clone(), name, span)?;
            }
        }
        TypeName::Array(param_inner) => {
            if let TypeName::Array(arg_inner) = arg_type {
                collect_generic_type_inferences(
                    param_inner,
                    arg_inner,
                    type_params,
                    inferred,
                    span,
                )?;
            }
        }
        TypeName::GenericInstance {
            name: param_name,
            args: param_args,
        } => {
            if let TypeName::GenericInstance {
                name: arg_name,
                args: arg_args,
            } = arg_type
                && param_name.eq_ignore_ascii_case(arg_name)
            {
                for (param_arg, arg_arg) in param_args.iter().zip(arg_args.iter()) {
                    collect_generic_type_inferences(
                        param_arg,
                        arg_arg,
                        type_params,
                        inferred,
                        span,
                    )?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn merge_inferred_type(
    slot: &mut Option<TypeName>,
    inferred_type: TypeName,
    param_name: &str,
    span: Span,
) -> Result<(), Diagnostic> {
    if let Some(existing) = slot {
        if !existing.same_type(&inferred_type) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                format!(
                    "Conflicting inferred types for '{}': '{}' and '{}'",
                    param_name,
                    existing.display_name(),
                    inferred_type.display_name()
                ),
                Some(span),
            ));
        }
    } else {
        *slot = Some(inferred_type);
    }
    Ok(())
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
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<TypeName, Diagnostic> {
    if object_type.same_type(&TypeName::Variant)
        || matches!(object_type, TypeName::User(name) if name.eq_ignore_ascii_case("Object"))
    {
        for arg in args {
            validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        }
        return Ok(TypeName::Variant);
    }
    let (class_name, bindings) = generic_bindings_for_type(object_type, types);
    if !matches!(
        object_type,
        TypeName::User(_) | TypeName::GenericInstance { .. }
    ) {
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, option_explicit),
        )? {
            return Ok(res_ty);
        }

        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires a class instance",
            Some(span),
        ));
    }
    let Some(class_sig) = types.get_class(&class_name) else {
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, option_explicit),
        )? {
            return Ok(res_ty);
        }

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
            context,
            option_explicit,
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
                "Function",
                method_sig,
                args,
                span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
            )?;
            return Ok(method_sig
                .return_type
                .clone()
                .expect("function return")
                .substitute_generics(&bindings));
        }
        if let Some(get) = class_sig
            .properties
            .get(&key(method))
            .and_then(|p| p.get.as_ref())
        {
            ensure_visible(get.visibility, &class_sig.name, method, current_class, span)?;
            let return_type = get
                .return_type
                .clone()
                .expect("property return type")
                .substitute_generics(&bindings);

            // Case 1: The property itself takes these arguments
            if get.params.len() == args.len() {
                // Try to validate arguments for the property Get
                let dummy_sig = CallableSig {
                    attributes: Vec::new(),
                    visibility: get.visibility,
                    name: method.to_string(),
                    type_params: Vec::new(),
                    generic_constraints: Vec::new(),
                    is_shared: false,
                    _is_iterator: get.is_iterator,
                    is_declare: false,
                    params: get.params.clone(),
                    return_type: Some(return_type.clone()),
                };
                if validate_arguments(
                    "Property",
                    &dummy_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
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
                    context,
                    option_explicit,
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
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, option_explicit),
        )? {
            return Ok(res_ty);
        }

        Err(unknown_class_member(class_sig, method, as_expression, span))
    } else {
        if let Some(method_sig) = class_sig.subs.get(&key(method)) {
            ensure_visible(
                method_sig.visibility,
                &class_sig.name,
                method,
                current_class,
                span,
            )?;
            validate_arguments(
                "Sub",
                method_sig,
                args,
                span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
            )?;
            return Ok(TypeName::Variant);
        }
        // Sub-style property call (e.g., obj.Prop = value or obj.Prop(idx) = value)
        // This is complex because MemberCall is usually for reads.
        // But some VBA code might use MemberCall as a statement for something that returns an object and then calls a default sub?
        // Actually MemberSubCall is used for subs.

        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, option_explicit),
        )? {
            return Ok(res_ty);
        }

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
        Err(unknown_class_member(class_sig, method, as_expression, span))
    }
}

fn unknown_class_member(
    class_sig: &ClassSig,
    method: &str,
    as_expression: bool,
    span: Span,
) -> Diagnostic {
    let noun = if as_expression {
        "method or property"
    } else {
        "method"
    };
    let candidates = class_member_names(class_sig);
    Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Method or property '{}' was not found on type '{}'",
            method, class_sig.name
        ),
        Some(span),
    )
    .with_primary_label(format!("unknown {noun}"))
    .with_name_suggestion(method, candidates.iter().map(String::as_str))
    .with_available_items("available members", candidates.iter().map(String::as_str))
}

fn class_member_names(class_sig: &ClassSig) -> Vec<String> {
    let mut names = Vec::new();
    names.extend(class_sig.subs.values().map(|sig| sig.name.clone()));
    names.extend(class_sig.functions.values().map(|sig| sig.name.clone()));
    names.extend(class_sig.properties.values().map(|sig| sig.name.clone()));
    names.extend(class_sig.events.values().map(|sig| sig.name.clone()));
    names
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
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<TypeName, Diagnostic> {
    let (type_name, bindings) = generic_bindings_for_type(object_type, types);
    if let Some(interface_sig) = types.get_interface(&type_name) {
        if as_expression {
            if let Some(method_sig) = interface_sig.functions.get(&key(method)) {
                validate_arguments(
                    "Function",
                    method_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                return Ok(method_sig
                    .return_type
                    .clone()
                    .expect("function return")
                    .substitute_generics(&bindings));
            }
            if let Some(get) = interface_sig
                .properties
                .get(&key(method))
                .and_then(|p| p.get.as_ref())
            {
                let return_type = get
                    .return_type
                    .clone()
                    .expect("property return type")
                    .substitute_generics(&bindings);
                let dummy_sig = CallableSig {
                    attributes: Vec::new(),
                    visibility: get.visibility,
                    name: method.to_string(),
                    type_params: Vec::new(),
                    generic_constraints: Vec::new(),
                    is_shared: false,
                    _is_iterator: get.is_iterator,
                    is_declare: false,
                    params: get.params.clone(),
                    return_type: Some(return_type.clone()),
                };
                validate_arguments(
                    "Property",
                    &dummy_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                return Ok(return_type);
            }
            if interface_sig.subs.contains_key(&key(method)) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Sub method '{}' cannot be used as an expression", method),
                    Some(span),
                ));
            }
        } else {
            if let Some(method_sig) = interface_sig.subs.get(&key(method)) {
                validate_arguments(
                    "Sub",
                    method_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                return Ok(TypeName::Variant);
            }
            if let Some(property_accessor) = interface_sig
                .properties
                .get(&key(method))
                .and_then(|p| p.let_.as_ref().or(p.set.as_ref()))
            {
                let dummy_sig = CallableSig {
                    attributes: Vec::new(),
                    visibility: property_accessor.visibility,
                    name: method.to_string(),
                    type_params: Vec::new(),
                    generic_constraints: Vec::new(),
                    is_shared: false,
                    _is_iterator: property_accessor.is_iterator,
                    is_declare: false,
                    params: property_accessor.params.clone(),
                    return_type: None,
                };
                validate_arguments(
                    "Property",
                    &dummy_sig,
                    args,
                    span,
                    ExprValidation::new(symbols, types, signatures, context, option_explicit),
                )?;
                return Ok(TypeName::Variant);
            }
            if interface_sig.functions.contains_key(&key(method)) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Function method '{}' cannot be used as a Sub", method),
                    Some(span),
                ));
            }
        }
    }

    let type_sig = types.get(&type_name).ok_or_else(|| {
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
                "Function",
                method_sig,
                args,
                span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
            )?;
            return Ok(method_sig
                .return_type
                .clone()
                .expect("function return")
                .substitute_generics(&bindings));
        }
        if let Some(get) = type_sig
            .properties
            .get(&key(method))
            .and_then(|p| p.get.as_ref())
        {
            ensure_visible(get.visibility, &type_sig.name, method, current_type, span)?;
            let return_type = get
                .return_type
                .clone()
                .expect("property return type")
                .substitute_generics(&bindings);
            let dummy_sig = CallableSig {
                attributes: Vec::new(),
                visibility: get.visibility,
                name: method.to_string(),
                type_params: Vec::new(),
                generic_constraints: Vec::new(),
                is_shared: false,
                _is_iterator: get.is_iterator,
                is_declare: false,
                params: get.params.clone(),
                return_type: Some(return_type.clone()),
            };
            validate_arguments(
                "Property",
                &dummy_sig,
                args,
                span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
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
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, option_explicit),
        )? {
            return Ok(res_ty);
        }

        Err(unknown_structure_member(
            type_sig,
            method,
            as_expression,
            span,
        ))
    } else {
        if method.eq_ignore_ascii_case("New")
            || method.eq_ignore_ascii_case("Constructor")
            || method.eq_ignore_ascii_case("Initialize")
        {
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
            validate_arguments(
                "Sub",
                method_sig,
                args,
                span,
                ExprValidation::new(symbols, types, signatures, context, option_explicit),
            )?;
            return Ok(TypeName::Variant);
        }
        if let Some(res_ty) = resolve_extension_method(
            object_type,
            method,
            args,
            as_expression,
            span,
            ExprValidation::new(symbols, types, signatures, context, option_explicit),
        )? {
            return Ok(res_ty);
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
        Err(unknown_structure_member(
            type_sig,
            method,
            as_expression,
            span,
        ))
    }
}

fn unknown_structure_member(
    type_sig: &TypeSig,
    method: &str,
    as_expression: bool,
    span: Span,
) -> Diagnostic {
    let noun = if as_expression {
        "method or property"
    } else {
        "method"
    };
    let candidates = structure_member_names(type_sig);
    Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Method or property '{}' was not found on type '{}'",
            method, type_sig.name
        ),
        Some(span),
    )
    .with_primary_label(format!("unknown {noun}"))
    .with_name_suggestion(method, candidates.iter().map(String::as_str))
    .with_available_items("available members", candidates.iter().map(String::as_str))
}

fn structure_member_names(type_sig: &TypeSig) -> Vec<String> {
    let mut names = Vec::new();
    names.extend(type_sig.subs.values().map(|sig| sig.name.clone()));
    names.extend(type_sig.functions.values().map(|sig| sig.name.clone()));
    names.extend(type_sig.properties.values().map(|sig| sig.name.clone()));
    names
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
    if object_type.same_type(&TypeName::Variant)
        || matches!(object_type, TypeName::User(name) if name.eq_ignore_ascii_case("Object"))
    {
        return Ok(TypeName::Variant);
    }
    let (type_name, bindings) = generic_bindings_for_type(object_type, types);
    if !matches!(
        object_type,
        TypeName::User(_) | TypeName::GenericInstance { .. }
    ) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    }

    if let Some(type_sig) = types.get(&type_name) {
        if let Some(field) = type_sig.fields.get(&key(member)) {
            ensure_visible(
                field.visibility,
                &type_sig.name,
                member,
                current_class,
                span,
            )?;
            return Ok(field.ty.substitute_generics(&bindings));
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
        return Ok(get
            .return_type
            .clone()
            .expect("get return type")
            .substitute_generics(&bindings));
    }

    let class_sig = types.get_class(&type_name).ok_or_else(|| {
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
        return Ok(field_sig.ty.substitute_generics(&bindings));
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
    Ok(get
        .return_type
        .clone()
        .expect("get return type")
        .substitute_generics(&bindings))
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
    let (type_name, bindings) = generic_bindings_for_type(object_type, types);
    if !matches!(
        object_type,
        TypeName::User(_) | TypeName::GenericInstance { .. }
    ) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    }

    if let Some(type_sig) = types.get(&type_name) {
        if let Some(field) = type_sig.fields.get(&key(member)) {
            ensure_visible(
                field.visibility,
                &type_sig.name,
                member,
                current_class,
                span,
            )?;
            return Ok(field.ty.substitute_generics(&bindings));
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
        return Ok(accessor.params[0].ty.substitute_generics(&bindings));
    }

    let class_sig = types.get_class(&type_name).ok_or_else(|| {
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
        return Ok(field_sig.ty.substitute_generics(&bindings));
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
    Ok(accessor.params[0].ty.substitute_generics(&bindings))
}

fn ensure_visible(
    visibility: Visibility,
    owner_class: &str,
    member: &str,
    current_class: Option<&str>,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if visibility == Visibility::Public
        || visibility == Visibility::Friend
        || visibility == Visibility::ProtectedFriend
        || current_class.is_some_and(|class_name| class_name.eq_ignore_ascii_case(owner_class))
    {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE,
            format!(
                "Member '{}' is {} in Class '{}'",
                member,
                if visibility == Visibility::Protected {
                    "Protected"
                } else {
                    "Private"
                },
                owner_class
            ),
            Some(span),
        )
        .with_primary_label("member is not accessible here")
        .with_help("access this member from an allowed class scope or make it Public"))
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
            if types.generic_params.contains(&key(name))
                || name.eq_ignore_ascii_case("Object")
                || name.eq_ignore_ascii_case("Collection")
            {
                Ok(())
            } else if let Some(sig) = types.get(name)
                && !sig.type_params.is_empty()
            {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Generic type '{}' requires {} type argument(s)",
                        sig.name,
                        sig.type_params.len()
                    ),
                    Some(span),
                ))
            } else if let Some(sig) = types.get_class(name)
                && !sig.type_params.is_empty()
            {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Generic type '{}' requires {} type argument(s)",
                        sig.name,
                        sig.type_params.len()
                    ),
                    Some(span),
                ))
            } else if let Some(sig) = types.get_interface(name)
                && !sig.type_params.is_empty()
            {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Generic type '{}' requires {} type argument(s)",
                        sig.name,
                        sig.type_params.len()
                    ),
                    Some(span),
                ))
            } else if types.contains(name) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Type '{}' is not defined", name),
                    Some(span),
                ))
            }
        }
        TypeName::GenericInstance { name, args } => {
            let expected = types
                .get(name)
                .map(|sig| (&sig.name, &sig.type_params, &sig.generic_constraints))
                .or_else(|| {
                    types
                        .get_class(name)
                        .map(|sig| (&sig.name, &sig.type_params, &sig.generic_constraints))
                })
                .or_else(|| {
                    types
                        .get_interface(name)
                        .map(|sig| (&sig.name, &sig.type_params, &sig.generic_constraints))
                });
            let Some((canonical, type_params, generic_constraints)) = expected else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Type '{}' is not defined", name),
                    Some(span),
                ));
            };
            let expected_count = type_params.len();
            if expected_count != args.len() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Type parameter count mismatch for {}. Expected {}, received {}",
                        canonical,
                        expected_count,
                        args.len()
                    ),
                    Some(span),
                )
                .with_help(format!("received {}", ty.display_name())));
            }
            for arg in args {
                ensure_known_type(arg, types, span)?;
            }
            validate_generic_constraints(
                canonical,
                type_params,
                generic_constraints,
                args,
                types,
                span,
            )?;
            Ok(())
        }
        TypeName::Enum(name) => {
            if types.enums.contains_key(&key(name)) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Enum '{}' is not defined", name),
                    Some(span),
                ))
            }
        }
        TypeName::Array(inner) => ensure_known_type(inner, types, span),
    }
}

fn validate_generic_constraints(
    owner: &str,
    type_params: &[String],
    constraints: &[crate::GenericParamConstraint],
    type_args: &[TypeName],
    types: &TypeRegistry,
    span: Span,
) -> Result<(), Diagnostic> {
    for constraint in constraints {
        let Some(index) = type_params
            .iter()
            .position(|param| param.eq_ignore_ascii_case(&constraint.name))
        else {
            continue;
        };
        let arg = &type_args[index];
        println!(
            "Validating constraint for '{}': require_new={}",
            constraint.name, constraint.require_new
        );
        if constraint.require_class && !is_reference_type(arg, types) {
            return Err(generic_constraint_error(
                owner,
                &constraint.name,
                arg,
                "must be a reference type",
                span,
            ));
        }
        if constraint.require_structure && !is_value_type(arg, types) {
            return Err(generic_constraint_error(
                owner,
                &constraint.name,
                arg,
                "must be a value type",
                span,
            ));
        }
        if constraint.require_new && !has_parameterless_constructor(arg, types) {
            println!("  Constraint FAILED: require_new check failed");
            return Err(generic_constraint_error(
                owner,
                &constraint.name,
                arg,
                "must have a public parameterless constructor",
                span,
            ));
        }
        for bound in &constraint.bounds {
            if !satisfies_type_bound(arg, bound, types) {
                return Err(generic_constraint_error(
                    owner,
                    &constraint.name,
                    arg,
                    &format!("must inherit from or implement '{}'", bound.display_name()),
                    span,
                ));
            }
        }
    }
    Ok(())
}

fn generic_constraint_error(
    owner: &str,
    param: &str,
    arg: &TypeName,
    requirement: &str,
    span: Span,
) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
        format!(
            "Type argument '{}' for '{}.{}' {}",
            arg.display_name(),
            owner,
            param,
            requirement
        ),
        Some(span),
    )
}

fn is_reference_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    match ty {
        TypeName::String | TypeName::Array(_) => true,
        TypeName::User(name) => {
            name.eq_ignore_ascii_case("Object")
                || name.eq_ignore_ascii_case("Collection")
                || types.get_class(name).is_some()
                || types.get_interface(name).is_some()
        }
        TypeName::GenericInstance { name, .. } => {
            types.get_class(name).is_some() || types.get_interface(name).is_some()
        }
        _ => false,
    }
}

fn is_value_type(ty: &TypeName, types: &TypeRegistry) -> bool {
    match ty {
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
        | TypeName::Boolean
        | TypeName::Date
        | TypeName::Ptr
        | TypeName::FuncPtr
        | TypeName::Enum(_) => true,
        TypeName::User(name) => {
            types.get(name).is_some_and(|sig| sig.is_structure) || types.get_enum(name).is_some()
        }
        TypeName::GenericInstance { name, .. } => {
            types.get(name).is_some_and(|sig| sig.is_structure)
        }
        TypeName::String | TypeName::Variant | TypeName::Array(_) => false,
    }
}

fn has_parameterless_constructor(ty: &TypeName, types: &TypeRegistry) -> bool {
    if is_value_type(ty, types) {
        return true;
    }
    match ty {
        TypeName::User(name) if name.eq_ignore_ascii_case("Object") => true,
        TypeName::User(name) => types
            .get_class(name)
            .is_some_and(class_has_public_default_new),
        TypeName::GenericInstance { name, .. } => types
            .get_class(name)
            .is_some_and(class_has_public_default_new),
        _ => false,
    }
}

fn class_has_public_default_new(class: &ClassSig) -> bool {
    if class.inheritance == crate::ClassInheritance::MustInherit {
        return false;
    }

    // Check for explicit constructor (Sub New or Class_Initialize)
    let ctor = class
        .subs
        .get("initialize")
        .or_else(|| class.subs.get("class_initialize"));

    match ctor {
        Some(init) => init.visibility == Visibility::Public && init.params.is_empty(),
        None => true, // No explicit constructor means public default
    }
}

fn satisfies_type_bound(arg: &TypeName, bound: &TypeName, types: &TypeRegistry) -> bool {
    if arg.same_type(bound) {
        return true;
    }
    match (arg, bound) {
        (_, TypeName::User(name)) if name.eq_ignore_ascii_case("Object") => {
            is_reference_type(arg, types)
        }
        (TypeName::User(arg_name), TypeName::User(bound_name))
        | (TypeName::GenericInstance { name: arg_name, .. }, TypeName::User(bound_name)) => {
            class_inherits_from(arg_name, bound_name, types)
        }
        _ => false,
    }
}

fn class_inherits_from(class_name: &str, bound_name: &str, types: &TypeRegistry) -> bool {
    let Some(class) = types.get_class(class_name) else {
        return false;
    };
    let Some(base) = &class.base_class else {
        return false;
    };
    let (TypeName::User(base_name)
    | TypeName::GenericInstance {
        name: base_name, ..
    }) = base
    else {
        return false;
    };
    base_name.eq_ignore_ascii_case(bound_name) || class_inherits_from(base_name, bound_name, types)
}

fn generic_bindings_for_type(
    ty: &TypeName,
    types: &TypeRegistry,
) -> (String, Vec<(String, TypeName)>) {
    match ty {
        TypeName::GenericInstance { name, args } => {
            let params = types
                .get(name)
                .map(|sig| sig.type_params.clone())
                .or_else(|| types.get_class(name).map(|sig| sig.type_params.clone()))
                .or_else(|| types.get_interface(name).map(|sig| sig.type_params.clone()))
                .unwrap_or_default();
            (
                name.clone(),
                params.into_iter().zip(args.iter().cloned()).collect(),
            )
        }
        TypeName::User(name) => (name.clone(), Vec::new()),
        _ => (ty.display_name(), Vec::new()),
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
        if target.same_type(&TypeName::Variant) {
            return Ok(());
        }
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

    if let (TypeName::User(src_name), _) = (source, target)
        && (class_inherits_from(src_name, &target.display_name(), types)
            || class_implements_interface(src_name, target, types))
    {
        return Ok(());
    }

    if let (TypeName::GenericInstance { name: src_name, .. }, _) = (source, target)
        && (class_inherits_from(src_name, &target.display_name(), types)
            || class_implements_interface(src_name, target, types))
    {
        return Ok(());
    }

    ensure_assignable(target, source, span)
}

fn class_implements_interface(
    class_name: &str,
    interface_ty: &TypeName,
    types: &TypeRegistry,
) -> bool {
    let Some(class) = types.get_class(class_name) else {
        return false;
    };
    for impl_ty in &class.implements {
        if impl_ty.same_type(interface_ty) {
            return true;
        }
    }
    if let Some(base) = &class.base_class {
        let base_name = match base {
            TypeName::User(name) => name,
            TypeName::GenericInstance { name, .. } => name,
            _ => return false,
        };
        if class_implements_interface(base_name, interface_ty, types) {
            return true;
        }
    }
    false
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
    match ty {
        TypeName::User(name) => {
            types.get_class(name).is_some()
                || types.get_interface(name).is_some()
                || name.eq_ignore_ascii_case("Object")
                || name.eq_ignore_ascii_case("Collection")
        }
        TypeName::GenericInstance { name, .. } => {
            types.get_class(name).is_some() || types.get_interface(name).is_some()
        }
        _ => false,
    }
}

fn find_overloaded_binary_operator(
    left: &TypeName,
    op: crate::OperatorKind,
    right: &TypeName,
    types: &TypeRegistry,
) -> Option<TypeName> {
    // Try left operand
    if let TypeName::User(name) = left {
        if let Some(class) = types.get_class(name)
            && let Some(operator) = class.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
        if let Some(type_sig) = types.get(name)
            && let Some(operator) = type_sig.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
    }

    // Try right operand
    if let TypeName::User(name) = right {
        if let Some(class) = types.get_class(name)
            && let Some(operator) = class.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
        if let Some(type_sig) = types.get(name)
            && let Some(operator) = type_sig.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
    }

    None
}

fn find_overloaded_unary_operator(
    op: crate::OperatorKind,
    ty: &TypeName,
    types: &TypeRegistry,
) -> Option<TypeName> {
    if let TypeName::User(name) = ty {
        if let Some(class) = types.get_class(name)
            && let Some(operator) = class.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
        if let Some(type_sig) = types.get(name)
            && let Some(operator) = type_sig.operators.get(&op)
        {
            return Some(operator.return_type.clone().unwrap_or(TypeName::Variant));
        }
    }
    None
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
    context: &Context<'_>,
    option_explicit: bool,
) -> Result<(), Diagnostic> {
    match item {
        CaseItem::Value(value) => {
            let value_type =
                validate_expr(value, symbols, types, signatures, context, option_explicit)?;
            ensure_case_comparable(subject_type, &value_type, value.span)
        }
        CaseItem::Range { start, end } => {
            let start_type =
                validate_expr(start, symbols, types, signatures, context, option_explicit)?;
            let end_type =
                validate_expr(end, symbols, types, signatures, context, option_explicit)?;
            ensure_case_comparable(subject_type, &start_type, start.span)?;
            ensure_case_comparable(subject_type, &end_type, end.span)?;
            ensure_case_orderable(subject_type, start.span)
        }
        CaseItem::Compare { op, expr } => {
            let expr_type =
                validate_expr(expr, symbols, types, signatures, context, option_explicit)?;
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
        || is_inherited_class_assignable(target, source)
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

fn is_inherited_class_assignable(target: &TypeName, source: &TypeName) -> bool {
    matches!((target, source), (TypeName::User(_), TypeName::User(_)))
}

fn validate_err_raise_args(
    args: &[Expr],
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
    signatures: &Signatures,
    span: crate::runtime::Span,
    context: &Context<'_>,
    option_explicit: bool,
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
        let actual = validate_expr(arg, symbols, types, signatures, context, option_explicit)?;
        ensure_assignable(&expected[index], &actual, arg.span)?;
    }
    Ok(())
}

fn resolve_extension_method(
    object_type: &TypeName,
    method: &str,
    args: &[Expr],
    as_expression: bool,
    span: crate::runtime::Span,
    validation: ExprValidation<'_, '_>,
) -> Result<Option<TypeName>, Diagnostic> {
    let type_key = object_type.display_name().to_lowercase();
    if let Some(methods) = validation.signatures.extension_methods.get(&type_key) {
        for sig in methods {
            if sig.name.eq_ignore_ascii_case(method) {
                if sig.params.is_empty() {
                    continue;
                }

                let mut shifted_sig = sig.clone();
                shifted_sig.params.remove(0);

                validate_arguments(
                    if as_expression { "Function" } else { "Sub" },
                    &shifted_sig,
                    args,
                    span,
                    validation,
                )?;

                if as_expression {
                    return Ok(Some(sig.return_type.clone().unwrap_or(TypeName::Variant)));
                } else {
                    return Ok(Some(TypeName::Variant));
                }
            }
        }
    }
    Ok(None)
}
