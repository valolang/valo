use super::*;

pub(super) fn collect_types(program: &Program) -> Result<TypeRegistry, Diagnostic> {
    let mut types = HashMap::new();
    let mut enums = HashMap::new();
    let mut classes = HashMap::new();

    // Add built-in Error class
    classes.insert(
        key("Error"),
        ClassSig {
            name: "Error".to_string(),
            fields: {
                let mut f = HashMap::new();
                f.insert(
                    key("Number"),
                    ClassFieldSig {
                        visibility: Visibility::Public,
                        with_events: false,
                        ty: TypeName::Integer,
                        array: None,
                    },
                );
                f.insert(
                    key("Message"),
                    ClassFieldSig {
                        visibility: Visibility::Public,
                        with_events: false,
                        ty: TypeName::String,
                        array: None,
                    },
                );
                f.insert(
                    key("Description"),
                    ClassFieldSig {
                        visibility: Visibility::Public,
                        with_events: false,
                        ty: TypeName::String,
                        array: None,
                    },
                );
                f.insert(
                    key("Source"),
                    ClassFieldSig {
                        visibility: Visibility::Public,
                        with_events: false,
                        ty: TypeName::String,
                        array: None,
                    },
                );
                f.insert(
                    key("HelpFile"),
                    ClassFieldSig {
                        visibility: Visibility::Public,
                        with_events: false,
                        ty: TypeName::String,
                        array: None,
                    },
                );
                f.insert(
                    key("HelpContext"),
                    ClassFieldSig {
                        visibility: Visibility::Public,
                        with_events: false,
                        ty: TypeName::Integer,
                        array: None,
                    },
                );
                f
            },
            events: HashMap::new(),
            subs: HashMap::new(),
            functions: HashMap::new(),
            properties: HashMap::new(),
            default_property: None,
        },
    );

    for type_decl in &program.types {
        let type_key = key(&type_decl.name);
        if types.contains_key(&type_key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Type '{}' is already defined", type_decl.name),
                Some(type_decl.span),
            ));
        }

        let mut fields = HashMap::new();
        for field in &type_decl.fields {
            let field_key = key(&field.name);
            if fields.contains_key(&field_key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!(
                        "Field '{}' is already declared in Type '{}'",
                        field.name, type_decl.name
                    ),
                    Some(field.span),
                ));
            }
            fields.insert(
                field_key,
                FieldSig {
                    ty: field.ty.clone(),
                },
            );
        }

        types.insert(
            type_key,
            TypeSig {
                name: type_decl.name.clone(),
                fields,
            },
        );
    }

    for enum_decl in &program.enums {
        let enum_key = key(&enum_decl.name);
        if types.contains_key(&enum_key) || enums.contains_key(&enum_key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Enum '{}' is already defined", enum_decl.name),
                Some(enum_decl.span),
            ));
        }
        let mut members = HashMap::new();
        let mut previous = -1;
        for member in &enum_decl.members {
            let member_key = key(&member.name);
            if members.contains_key(&member_key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!(
                        "Enum member '{}' is already declared in Enum '{}'",
                        member.name, enum_decl.name
                    ),
                    Some(member.span),
                ));
            }
            let value = if let Some(expr) = &member.value {
                eval_enum_const_expr(expr, &members)?
            } else {
                previous + 1
            };
            previous = value;
            members.insert(member_key, value);
        }
        enums.insert(
            enum_key,
            EnumSig {
                name: enum_decl.name.clone(),
                members,
            },
        );
    }

    for class_decl in &program.classes {
        let class_key = key(&class_decl.name);
        if types.contains_key(&class_key)
            || enums.contains_key(&class_key)
            || classes.contains_key(&class_key)
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Class '{}' is already defined", class_decl.name),
                Some(class_decl.span),
            ));
        }

        let mut fields = HashMap::new();
        let mut events = HashMap::new();
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut properties: HashMap<String, ClassPropertySig> = HashMap::new();
        let mut default_member: Option<String> = None;
        let mut constructor_span = None;
        let mut terminator_span = None;
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    let field_key = key(&field.name);
                    if fields.contains_key(&field_key)
                        || events.contains_key(&field_key)
                        || properties.contains_key(&field_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Field '{}' conflicts with another member in Class '{}'",
                                field.name, class_decl.name
                            ),
                            Some(field.span),
                        ));
                    }
                    fields.insert(
                        field_key,
                        ClassFieldSig {
                            visibility: field.visibility,
                            with_events: field.with_events,
                            ty: field.ty.clone(),
                            array: field.array.clone(),
                        },
                    );
                }
                ClassMember::Event(event) => {
                    let event_key = key(&event.name);
                    if fields.contains_key(&event_key)
                        || events.contains_key(&event_key)
                        || subs.contains_key(&event_key)
                        || functions.contains_key(&event_key)
                        || properties.contains_key(&event_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Event '{}' conflicts with another member in Class '{}'",
                                event.name, class_decl.name
                            ),
                            Some(event.span),
                        ));
                    }
                    events.insert(
                        event_key,
                        ClassEventSig {
                            visibility: event.visibility,
                            name: event.name.clone(),
                            params: params_to_sigs(&event.params),
                            return_type: None,
                        },
                    );
                }
                ClassMember::Sub(method) => {
                    let method_key = key(&method.procedure.name);
                    if is_constructor_name(&method_key) {
                        if constructor_span.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has duplicate constructor definitions; use only one of Constructor, Initialize, or Class_Initialize",
                                    class_decl.name
                                ),
                                Some(method.procedure.span),
                            ));
                        }
                        constructor_span = Some(method.procedure.span);
                    }
                    if is_terminator_name(&method_key) {
                        if terminator_span.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has duplicate terminator definitions; use only one of Terminate or Class_Terminate",
                                    class_decl.name
                                ),
                                Some(method.procedure.span),
                            ));
                        }
                        terminator_span = Some(method.procedure.span);
                    }
                    if subs.contains_key(&method_key)
                        || events.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Method '{}' conflicts with another member in Class '{}'",
                                method.procedure.name, class_decl.name
                            ),
                            Some(method.procedure.span),
                        ));
                    }
                    subs.insert(
                        method_key,
                        ClassMethodSig {
                            visibility: method.visibility,
                            name: method.procedure.name.clone(),
                            params: params_to_sigs(&method.procedure.params),
                            return_type: None,
                        },
                    );
                }
                ClassMember::Function(method) => {
                    let method_key = key(&method.function.name);
                    if subs.contains_key(&method_key)
                        || events.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Method '{}' conflicts with another member in Class '{}'",
                                method.function.name, class_decl.name
                            ),
                            Some(method.function.span),
                        ));
                    }
                    functions.insert(
                        method_key,
                        ClassMethodSig {
                            visibility: method.visibility,
                            name: method.function.name.clone(),
                            params: params_to_sigs(&method.function.params),
                            return_type: Some(method.function.return_type.clone()),
                        },
                    );
                }
                ClassMember::Property(property) => {
                    let property_key = key(&property.name);
                    if property.is_default {
                        if property.kind != PropertyKind::Get {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!(
                                    "Only Property Get can be marked as Default in Class '{}'",
                                    class_decl.name
                                ),
                                Some(property.span),
                            ));
                        }
                        if default_member.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!("Class '{}' has multiple default members", class_decl.name),
                                Some(property.span),
                            ));
                        }
                        default_member = Some(property.name.clone());
                    }
                    if fields.contains_key(&property_key)
                        || events.contains_key(&property_key)
                        || subs.contains_key(&property_key)
                        || functions.contains_key(&property_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Property '{}' conflicts with another member in Class '{}'",
                                property.name, class_decl.name
                            ),
                            Some(property.span),
                        ));
                    }
                    let property_sig =
                        properties
                            .entry(property_key)
                            .or_insert_with(|| ClassPropertySig {
                                name: property.name.clone(),
                                get: None,
                                let_: None,
                                set: None,
                            });
                    let target = match property.kind {
                        PropertyKind::Get => &mut property_sig.get,
                        PropertyKind::Let => &mut property_sig.let_,
                        PropertyKind::Set => &mut property_sig.set,
                    };
                    if target.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                            format!(
                                "Property {:?} '{}' is already declared in Class '{}'",
                                property.kind, property.name, class_decl.name
                            ),
                            Some(property.span),
                        ));
                    }
                    *target = Some(PropertyAccessorSig {
                        visibility: property.visibility,
                        params: params_to_sigs(&property.params),
                        return_type: property.return_type.clone(),
                    });
                }
            }
        }
        classes.insert(
            class_key,
            ClassSig {
                name: class_decl.name.clone(),
                fields,
                events,
                subs,
                functions,
                properties,
                default_property: default_member,
            },
        );
    }

    let registry = TypeRegistry {
        types,
        enums,
        classes,
    };
    for type_decl in &program.types {
        for field in &type_decl.fields {
            ensure_known_type(&field.ty, &registry, field.span)?;
        }
    }
    for class_decl in &program.classes {
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => ensure_known_type(&field.ty, &registry, field.span)?,
                ClassMember::Event(event) => {
                    for param in &event.params {
                        ensure_known_type(&param.ty, &registry, param.span)?;
                    }
                }
                ClassMember::Sub(method) => {
                    for param in &method.procedure.params {
                        ensure_known_type(&param.ty, &registry, param.span)?;
                    }
                }
                ClassMember::Function(method) => {
                    ensure_known_type(
                        &method.function.return_type,
                        &registry,
                        method.function.span,
                    )?;
                    for param in &method.function.params {
                        ensure_known_type(&param.ty, &registry, param.span)?;
                    }
                }
                ClassMember::Property(property) => match property.kind {
                    PropertyKind::Get => {
                        for param in &property.params {
                            ensure_known_type(&param.ty, &registry, param.span)?;
                        }
                        ensure_known_type(
                            property.return_type.as_ref().expect("get return type"),
                            &registry,
                            property.span,
                        )?;
                    }
                    PropertyKind::Let | PropertyKind::Set => {
                        if property.params.is_empty() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!(
                                    "Property {:?} '{}' must have at least one parameter",
                                    property.kind, property.name
                                ),
                                Some(property.span),
                            ));
                        }
                        for param in &property.params {
                            ensure_known_type(&param.ty, &registry, param.span)?;
                        }
                        let last_param = property.params.last().unwrap();
                        if last_param.mode != PassingMode::ByVal {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                format!(
                                    "Property {:?} '{}' value parameter must be ByVal",
                                    property.kind, property.name
                                ),
                                Some(last_param.span),
                            ));
                        }
                        if property.kind == PropertyKind::Set
                            && !matches!(&last_param.ty, TypeName::User(name) if registry.get_class(name).is_some() || name.eq_ignore_ascii_case("Object"))
                        {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                format!(
                                    "Property Set '{}' value parameter must be a class type",
                                    property.name
                                ),
                                Some(last_param.span),
                            ));
                        }
                        if property.kind == PropertyKind::Let
                            && matches!(&last_param.ty, TypeName::User(name) if registry.get_class(name).is_some())
                        {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!(
                                    "Property Let '{}' value parameter cannot be a class type",
                                    property.name
                                ),
                                Some(last_param.span),
                            ));
                        }
                    }
                },
            }
        }
    }
    validate_withevents_handlers(program, &registry)?;

    Ok(registry)
}

fn is_constructor_name(name: &str) -> bool {
    name == "initialize" || name == "class_initialize"
}

fn is_terminator_name(name: &str) -> bool {
    name == "terminate" || name == "class_terminate"
}

fn validate_withevents_handlers(
    program: &Program,
    registry: &TypeRegistry,
) -> Result<(), Diagnostic> {
    for class_decl in &program.classes {
        let class_sig = registry
            .get_class(&class_decl.name)
            .expect("class signature collected");
        for member in &class_decl.members {
            let ClassMember::Field(field) = member else {
                continue;
            };
            let owner_field_sig = class_sig
                .fields
                .get(&key(&field.name))
                .expect("field collected");
            if !owner_field_sig.with_events {
                continue;
            }
            let TypeName::User(source_class_name) = &field.ty else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("WithEvents field '{}' must have a class type", field.name),
                    Some(field.span),
                ));
            };
            let Some(source_class) = registry.get_class(source_class_name) else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("WithEvents field '{}' must have a class type", field.name),
                    Some(field.span),
                ));
            };
            for event in source_class.events.values() {
                let handler_name = format!("{}_{}", field.name, event.name);
                let handler_key = key(&handler_name);
                if let Some(handler) = class_sig.functions.get(&handler_key) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!("Event handler '{}' must be a Sub method", handler.name),
                        Some(field.span),
                    ));
                }
                let Some(handler) = class_sig.subs.get(&handler_key) else {
                    continue;
                };
                if handler.params.len() != event.params.len()
                    || !handler
                        .params
                        .iter()
                        .zip(event.params.iter())
                        .all(|(left, right)| left.ty.same_type(&right.ty))
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!(
                            "Event handler '{}' signature does not match event '{}'",
                            handler.name, event.name
                        ),
                        Some(field.span),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn collect_signatures(
    program: &Program,
    types: &TypeRegistry,
) -> Result<Signatures, Diagnostic> {
    let mut subs = HashMap::new();
    let mut functions = HashMap::new();
    let mut names = HashMap::new();

    for type_decl in &program.types {
        names.insert(key(&type_decl.name), "Type");
    }
    for enum_decl in &program.enums {
        if let Some(existing) = names.insert(key(&enum_decl.name), "Enum") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!(
                    "Name '{}' conflicts with existing {}",
                    enum_decl.name, existing
                ),
                Some(enum_decl.span),
            ));
        }
    }
    for class_decl in &program.classes {
        names.insert(key(&class_decl.name), "Class");
    }

    for var in &program.module_vars {
        ensure_known_type(&var.ty, types, var.span)?;
        let name_key = key(&var.name);
        if let Some(existing) = names.insert(name_key, "module variable") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Name '{}' conflicts with existing {}", var.name, existing),
                Some(var.span),
            ));
        }
    }

    for const_decl in &program.module_consts {
        let name_key = key(&const_decl.name);
        if let Some(existing) = names.insert(name_key, "module constant") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!(
                    "Name '{}' conflicts with existing {}",
                    const_decl.name, existing
                ),
                Some(const_decl.span),
            ));
        }
    }

    for procedure in &program.procedures {
        validate_parameter_list(&procedure.params, types)?;

        let name_key = key(&procedure.name);
        if let Some(existing) = names.insert(name_key.clone(), "Sub") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!(
                    "Name '{}' conflicts with existing {}",
                    procedure.name, existing
                ),
                Some(procedure.span),
            ));
        }

        subs.insert(
            name_key,
            CallableSig {
                visibility: Visibility::Public,
                name: procedure.name.clone(),
                params: params_to_sigs(&procedure.params),
                return_type: None,
            },
        );
    }

    for function in &program.functions {
        validate_parameter_list(&function.params, types)?;
        ensure_known_type(&function.return_type, types, function.span)?;

        let name_key = key(&function.name);
        if let Some(existing) = names.insert(name_key.clone(), "Function") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!(
                    "Name '{}' conflicts with existing {}",
                    function.name, existing
                ),
                Some(function.span),
            ));
        }

        functions.insert(
            name_key,
            CallableSig {
                visibility: Visibility::Public,
                name: function.name.clone(),
                params: params_to_sigs(&function.params),
                return_type: Some(function.return_type.clone()),
            },
        );
    }

    Ok(Signatures { subs, functions })
}

fn validate_parameter_list(params: &[Parameter], types: &TypeRegistry) -> Result<(), Diagnostic> {
    let mut saw_optional = false;
    for (index, param) in params.iter().enumerate() {
        ensure_known_type(&param.ty, types, param.span)?;
        if param.is_param_array {
            if index + 1 != params.len() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "ParamArray must be the last parameter",
                    Some(param.span),
                ));
            }
            continue;
        }
        if param.is_optional {
            saw_optional = true;
            if let Some(default) = &param.optional_default {
                ensure_const_expr(default, &HashMap::new(), types)?;
            } else if !param.ty.same_type(&TypeName::Variant) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Optional parameters without defaults must be Variant",
                    Some(param.span),
                ));
            }
        } else if saw_optional {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Optional parameters must come after required parameters",
                Some(param.span),
            ));
        }
    }
    Ok(())
}

fn eval_enum_const_expr(expr: &Expr, members: &HashMap<String, i64>) -> Result<i64, Diagnostic> {
    match &expr.kind {
        ExprKind::Integer(value) => Ok(*value),
        ExprKind::Variable(name) => members.get(&key(name)).copied().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Enum member '{}' is not defined", name),
                Some(expr.span),
            )
        }),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => Ok(-eval_enum_const_expr(expr, members)?),
        ExprKind::Binary { left, op, right } => {
            let left = eval_enum_const_expr(left, members)?;
            let right = eval_enum_const_expr(right, members)?;
            match op {
                BinaryOp::Add => Ok(left + right),
                BinaryOp::Subtract => Ok(left - right),
                BinaryOp::Multiply => Ok(left * right),
                BinaryOp::Divide => {
                    if right == 0 {
                        Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            "Division by zero",
                            Some(expr.span),
                        ))
                    } else {
                        Ok(left / right)
                    }
                }
                _ => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Enum value expression must be numeric",
                    Some(expr.span),
                )),
            }
        }
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Enum value expression must be numeric",
            Some(expr.span),
        )),
    }
}

pub(super) fn collect_module_symbols(
    program: &Program,
    types: &TypeRegistry,
    signatures: &Signatures,
) -> Result<HashMap<String, VarType>, Diagnostic> {
    let mut symbols = HashMap::new();

    for var in &program.module_vars {
        ensure_known_type(&var.ty, types, var.span)?;
        let name_key = key(&var.name);
        if symbols.contains_key(&name_key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Module-level name '{}' is already declared", var.name),
                Some(var.span),
            ));
        }
        let var_type = if var.array.is_some() {
            VarType::Array(var.ty.clone())
        } else {
            VarType::Scalar(var.ty.clone())
        };
        symbols.insert(name_key, var_type);
    }

    for const_decl in &program.module_consts {
        ensure_const_expr(&const_decl.value, &symbols, types)?;
        let value_type = validate_expr(&const_decl.value, &symbols, types, signatures)?;
        let const_type = const_decl.ty.clone().unwrap_or(value_type.clone());
        ensure_known_type(&const_type, types, const_decl.span)?;
        ensure_assignable_expr(
            &const_type,
            &value_type,
            &const_decl.value,
            types,
            const_decl.span,
        )?;
        let name_key = key(&const_decl.name);
        if symbols.contains_key(&name_key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!(
                    "Module-level name '{}' is already declared",
                    const_decl.name
                ),
                Some(const_decl.span),
            ));
        }
        symbols.insert(name_key, VarType::Const(const_type));
    }

    Ok(symbols)
}

pub(super) fn ensure_const_expr(
    expr: &Expr,
    symbols: &HashMap<String, VarType>,
    types: &TypeRegistry,
) -> Result<(), Diagnostic> {
    match &expr.kind {
        ExprKind::String(_)
        | ExprKind::Integer(_)
        | ExprKind::Double(_)
        | ExprKind::Boolean(_)
        | ExprKind::Empty
        | ExprKind::Null => Ok(()),
        ExprKind::Variable(name) => {
            if symbols
                .get(&key(name))
                .is_some_and(|var_type| var_type.is_const())
                || enum_member_value_type(name, types).is_some()
            {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Const initializer must be a compile-time constant",
                    Some(expr.span),
                ))
            }
        }
        ExprKind::MemberAccess { object, field } => {
            if let ExprKind::Variable(enum_name) = &object.kind
                && types
                    .get_enum(enum_name)
                    .is_some_and(|enum_sig| enum_sig.members.contains_key(&key(field)))
            {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Const initializer must be a compile-time constant",
                    Some(expr.span),
                ))
            }
        }
        ExprKind::Unary { expr, .. } => ensure_const_expr(expr, symbols, types),
        ExprKind::Binary { left, right, .. } => {
            ensure_const_expr(left, symbols, types)?;
            ensure_const_expr(right, symbols, types)
        }
        ExprKind::Nothing
        | ExprKind::Missing
        | ExprKind::Me
        | ExprKind::WithTarget
        | ExprKind::New { .. }
        | ExprKind::Call { .. }
        | ExprKind::NamedArg { .. }
        | ExprKind::TypeOfIs { .. }
        | ExprKind::MemberCall { .. } => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Const initializer must be a compile-time constant",
            Some(expr.span),
        )),
    }
}

fn params_to_sigs(params: &[Parameter]) -> Vec<ParamSig> {
    params
        .iter()
        .map(|param| ParamSig {
            name: param.name.clone(),
            mode: param.mode,
            ty: param.ty.clone(),
            is_optional: param.is_optional,
            is_param_array: param.is_param_array,
        })
        .collect()
}

pub(super) fn validate_procedure(
    procedure: &Procedure,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    let mut symbols = HashMap::new();
    add_module_symbols(module_symbols, &mut symbols);
    add_parameters(&procedure.params, &mut symbols)?;
    validate_statements(
        &procedure.body,
        &mut symbols,
        types,
        signatures,
        Context::Sub,
        LoopContext::default(),
        false,
    )
}

pub(super) fn validate_function(
    function: &Function,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    let mut symbols = HashMap::new();
    add_module_symbols(module_symbols, &mut symbols);
    add_parameters(&function.params, &mut symbols)?;
    symbols.insert(
        key(&function.name),
        VarType::Scalar(function.return_type.clone()),
    );

    let mut saw_return = assigns_to_name(&function.body, &function.name);
    validate_statements(
        &function.body,
        &mut symbols,
        types,
        signatures,
        Context::Function {
            return_type: function.return_type.clone(),
            saw_return: &mut saw_return,
        },
        LoopContext::default(),
        false,
    )?;

    if !saw_return {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("Function '{}' must return a value", function.name),
            Some(function.span),
        ));
    }

    Ok(())
}

fn assigns_to_name(statements: &[Stmt], name: &str) -> bool {
    statements.iter().any(|stmt| match stmt {
        Stmt::Assign {
            target: crate::AssignTarget::Variable { name: target, .. },
            ..
        }
        | Stmt::SetAssign {
            target: crate::AssignTarget::Variable { name: target, .. },
            ..
        } => target.eq_ignore_ascii_case(name),
        Stmt::If {
            then_body,
            elseif_branches,
            else_body,
            ..
        } => {
            assigns_to_name(then_body, name)
                || elseif_branches
                    .iter()
                    .any(|branch| assigns_to_name(&branch.body, name))
                || assigns_to_name(else_body, name)
        }
        Stmt::SelectCase {
            branches,
            else_body,
            ..
        } => {
            branches
                .iter()
                .any(|branch| assigns_to_name(&branch.body, name))
                || assigns_to_name(else_body, name)
        }
        Stmt::While { body, .. }
        | Stmt::DoLoop { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. }
        | Stmt::With { body, .. } => assigns_to_name(body, name),
        _ => false,
    })
}

pub(super) fn add_parameters(
    params: &[Parameter],
    symbols: &mut HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    for param in params {
        let param_key = key(&param.name);
        if symbols.contains_key(&param_key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Parameter '{}' is already declared", param.name),
                Some(param.span),
            ));
        }
        let var_type = if param.is_param_array {
            VarType::Array(param.ty.clone())
        } else if param.is_optional {
            VarType::Optional(param.ty.clone())
        } else {
            VarType::Scalar(param.ty.clone())
        };
        symbols.insert(param_key, var_type);
    }
    Ok(())
}

pub(super) fn add_module_symbols(
    module_symbols: &HashMap<String, VarType>,
    symbols: &mut HashMap<String, VarType>,
) {
    for (name, var_type) in module_symbols {
        symbols.insert(name.clone(), var_type.clone());
    }
}
