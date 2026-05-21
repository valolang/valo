use super::*;
use crate::TypeKind;
use crate::runtime::Span;

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
            iterator: None,
            properties: HashMap::new(),
            enumerator: None,
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
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut properties: HashMap<String, ClassPropertySig> = HashMap::new();
        let mut default_member: Option<String> = None;
        let mut constructor_span = None;
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
                    visibility: field.visibility,
                    ty: field.ty.clone(),
                    array: field.array.clone(),
                },
            );
        }

        if type_decl.kind == TypeKind::Type && !type_decl.members.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Type declarations support fields only; use Structure for methods and properties",
                Some(type_decl.span),
            ));
        }

        for member in &type_decl.members {
            match member {
                ClassMember::Field(_) | ClassMember::Fields(_) | ClassMember::Const(_) => {
                    unreachable!("structure fields are stored separately")
                }
                ClassMember::Event(event) => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        "Structure cannot declare events",
                        Some(event.span),
                    ));
                }
                ClassMember::Iterator(method) => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        "Structure cannot declare Iterator members",
                        Some(method.function.span),
                    ));
                }
                ClassMember::Sub(method) => {
                    let method_key = key(&method.procedure.name);
                    if method_key == "terminate" || method_key == "class_terminate" {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            "Structure cannot declare Terminate or Class_Terminate",
                            Some(method.procedure.span),
                        ));
                    }
                    if method_key == "class_initialize" {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            "Structure cannot declare Class_Initialize; use Sub Constructor",
                            Some(method.procedure.span),
                        ));
                    }
                    if is_constructor_name(&method_key) {
                        if constructor_span.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Structure '{}' has duplicate constructor definitions",
                                    type_decl.name
                                ),
                                Some(method.procedure.span),
                            ));
                        }
                        constructor_span = Some(method.procedure.span);
                    }
                    if fields.contains_key(&method_key)
                        || subs.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Method '{}' conflicts with another member in Structure '{}'",
                                method.procedure.name, type_decl.name
                            ),
                            Some(method.procedure.span),
                        ));
                    }
                    subs.insert(
                        method_key,
                        ClassMethodSig {
                            visibility: method.visibility,
                            name: method.procedure.name.clone(),
                            _is_iterator: false,
                            is_declare: false,
                            params: params_to_sigs(&method.procedure.params),
                            return_type: None,
                        },
                    );
                }
                ClassMember::Function(method) => {
                    let method_key = key(&method.function.name);
                    if method_key == "constructor" || method_key == "initialize" {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            "Structure constructor must be declared as Sub Constructor",
                            Some(method.function.span),
                        ));
                    }
                    if fields.contains_key(&method_key)
                        || subs.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Method '{}' conflicts with another member in Structure '{}'",
                                method.function.name, type_decl.name
                            ),
                            Some(method.function.span),
                        ));
                    }
                    functions.insert(
                        method_key,
                        ClassMethodSig {
                            visibility: method.visibility,
                            name: method.function.name.clone(),
                            _is_iterator: method.function.is_iterator,
                            is_declare: false,
                            params: params_to_sigs(&method.function.params),
                            return_type: Some(method.function.return_type.clone()),
                        },
                    );
                }
                ClassMember::Property(property) => {
                    let property_key = key(&property.name);
                    if property.is_default {
                        if default_member
                            .as_ref()
                            .is_some_and(|name| !name.eq_ignore_ascii_case(&property.name))
                        {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Structure '{}' has multiple default members",
                                    type_decl.name
                                ),
                                Some(property.span),
                            ));
                        }
                        default_member = Some(property.name.clone());
                    }
                    if fields.contains_key(&property_key)
                        || subs.contains_key(&property_key)
                        || functions.contains_key(&property_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Property '{}' conflicts with another member in Structure '{}'",
                                property.name, type_decl.name
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
                                "Property {:?} '{}' is already declared in Structure '{}'",
                                property.kind, property.name, type_decl.name
                            ),
                            Some(property.span),
                        ));
                    }
                    *target = Some(PropertyAccessorSig {
                        visibility: property.visibility,
                        is_iterator: property.is_iterator,
                        params: params_to_sigs(&property.params),
                        return_type: property.return_type.clone(),
                    });
                }
            }
        }

        types.insert(
            type_key,
            TypeSig {
                name: type_decl.name.clone(),
                is_structure: type_decl.kind == TypeKind::Structure,
                fields,
                subs,
                functions,
                properties,
                default_property: default_member,
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
        let mut iterator: Option<ClassMethodSig> = None;
        let mut enumerator_member: Option<String> = None;
        let mut constructor_span = None;
        let mut terminator_span = None;
        let mut default_iterator_span: Option<Span> = None;
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => {
                    insert_class_field(class_decl, field, &mut fields, &events, &properties)?;
                }
                ClassMember::Fields(class_fields) => {
                    for field in class_fields {
                        insert_class_field(class_decl, field, &mut fields, &events, &properties)?;
                    }
                }
                ClassMember::Const(const_decl) => {
                    let const_key = key(&const_decl.name);
                    if fields.contains_key(&const_key)
                        || events.contains_key(&const_key)
                        || subs.contains_key(&const_key)
                        || functions.contains_key(&const_key)
                        || properties.contains_key(&const_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Const '{}' conflicts with another member in Class '{}'",
                                const_decl.name, class_decl.name
                            ),
                            Some(const_decl.span),
                        ));
                    }
                    fields.insert(
                        const_key,
                        ClassFieldSig {
                            visibility: const_decl.visibility,
                            with_events: false,
                            ty: const_decl.ty.clone().unwrap_or(TypeName::Variant),
                            array: None,
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
                            _is_iterator: false,
                            is_declare: false,
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
                                    "Class '{}' has duplicate constructor definitions; use only one of Sub New or Class_Initialize",
                                    class_decl.name
                                ),
                                Some(method.procedure.span),
                            ));
                        }
                        constructor_span = Some(method.procedure.span);
                    }
                    if is_terminator_name(&method_key) {
                        if !method.procedure.params.is_empty() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                "Terminate methods cannot declare parameters",
                                Some(method.procedure.span),
                            ));
                        }
                        if terminator_span.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has duplicate terminator definitions; use only one of Terminate, Sub Terminate, or Class_Terminate",
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
                            _is_iterator: false,
                            is_declare: false,
                            params: params_to_sigs(&method.procedure.params),
                            return_type: None,
                        },
                    );
                }
                ClassMember::Function(method) => {
                    let method_key = key(&method.function.name);
                    if method.is_enumerator {
                        if enumerator_member.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has multiple VB_UserMemId = -4 enumerator members",
                                    class_decl.name
                                ),
                                Some(method.function.span),
                            ));
                        }
                        enumerator_member = Some(method.function.name.clone());
                    }
                    if method.function.is_iterator && method.function.params.is_empty() {
                        if iterator.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has multiple default Iterator members",
                                    class_decl.name
                                ),
                                Some(method.function.span),
                            )
                            .with_secondary_label(
                                default_iterator_span.unwrap_or(method.function.span),
                                "previous iterator defined here",
                            ));
                        }
                        iterator = Some(ClassMethodSig {
                            visibility: method.visibility,
                            name: method.function.name.clone(),
                            _is_iterator: true,
                            is_declare: false,
                            params: vec![],
                            return_type: Some(method.function.return_type.clone()),
                        });
                        default_iterator_span = Some(method.function.span);
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
                            _is_iterator: method.function.is_iterator,
                            is_declare: false,
                            params: params_to_sigs(&method.function.params),
                            return_type: Some(method.function.return_type.clone()),
                        },
                    );
                }
                ClassMember::Iterator(method) => {
                    let method_key = key(&method.function.name);
                    if iterator.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                            format!(
                                "Class '{}' has multiple default Iterator members",
                                class_decl.name
                            ),
                            Some(method.function.span),
                        )
                        .with_secondary_label(
                            default_iterator_span.expect("iterator is some"),
                            "previous iterator defined here",
                        ));
                    }
                    if subs.contains_key(&method_key)
                        || events.contains_key(&method_key)
                        || functions.contains_key(&method_key)
                        || properties.contains_key(&method_key)
                        || fields.contains_key(&method_key)
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            format!(
                                "Iterator '{}' conflicts with another member in Class '{}'",
                                method.function.name, class_decl.name
                            ),
                            Some(method.function.span),
                        ));
                    }
                    iterator = Some(ClassMethodSig {
                        visibility: method.visibility,
                        name: method.function.name.clone(),
                        _is_iterator: true,
                        is_declare: false,
                        params: params_to_sigs(&method.function.params),
                        return_type: Some(method.function.return_type.clone()),
                    });
                    default_iterator_span = Some(method.function.span);
                }
                ClassMember::Property(property) => {
                    let property_key = key(&property.name);
                    if property.is_enumerator {
                        if property.kind != PropertyKind::Get {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!(
                                    "Only Property Get can be marked as VB_UserMemId = -4 in Class '{}'",
                                    class_decl.name
                                ),
                                Some(property.span),
                            ));
                        }
                        if enumerator_member.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has multiple VB_UserMemId = -4 enumerator members",
                                    class_decl.name
                                ),
                                Some(property.span),
                            ));
                        }
                        enumerator_member = Some(property.name.clone());
                    }
                    if property.is_default {
                        if default_member
                            .as_ref()
                            .is_some_and(|name| !name.eq_ignore_ascii_case(&property.name))
                        {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!("Class '{}' has multiple default members", class_decl.name),
                                Some(property.span),
                            ));
                        }
                        default_member = Some(property.name.clone());
                    }
                    if property.is_iterator
                        && property.params.is_empty()
                        && property.kind == PropertyKind::Get
                    {
                        if iterator.is_some() {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                                format!(
                                    "Class '{}' has multiple default Iterator members",
                                    class_decl.name
                                ),
                                Some(property.span),
                            )
                            .with_secondary_label(
                                default_iterator_span.expect("iterator is some"),
                                "previous iterator defined here",
                            ));
                        }
                        iterator = Some(ClassMethodSig {
                            visibility: property.visibility,
                            name: property.name.clone(),
                            _is_iterator: true,
                            is_declare: false,
                            params: vec![],
                            return_type: Some(property.return_type.clone().expect("get returns")),
                        });
                        default_iterator_span = Some(property.span);
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
                        is_iterator: property.is_iterator,
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
                iterator,
                properties,
                enumerator: enumerator_member,
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
            if let Some(initializer) = &field.initializer {
                ensure_const_expr(initializer, &HashMap::new(), &registry)?;
                let initializer_type = validate_expr(
                    initializer,
                    &HashMap::new(),
                    &registry,
                    &Signatures {
                        subs: HashMap::new(),
                        functions: HashMap::new(),
                    },
                )?;
                ensure_assignable_expr(
                    &field.ty,
                    &initializer_type,
                    initializer,
                    &registry,
                    initializer.span,
                )?;
            }
        }
        for member in &type_decl.members {
            match member {
                ClassMember::Field(_)
                | ClassMember::Fields(_)
                | ClassMember::Const(_)
                | ClassMember::Event(_)
                | ClassMember::Iterator(_) => {}
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
                    }
                },
            }
        }
    }
    for class_decl in &program.classes {
        for member in &class_decl.members {
            match member {
                ClassMember::Field(field) => validate_class_field_type(field, &registry)?,
                ClassMember::Fields(fields) => {
                    for field in fields {
                        validate_class_field_type(field, &registry)?;
                    }
                }
                ClassMember::Const(const_decl) => {
                    let mut symbols = HashMap::new();
                    ensure_const_expr(&const_decl.value, &symbols, &registry)?;
                    if let Some(ty) = &const_decl.ty {
                        ensure_known_type(ty, &registry, const_decl.span)?;
                    }
                    symbols.insert(
                        key(&const_decl.name),
                        VarType::Const(const_decl.ty.clone().unwrap_or(TypeName::Variant)),
                    );
                }
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
                ClassMember::Iterator(method) => {
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

fn insert_class_field(
    class_decl: &crate::ClassDecl,
    field: &crate::ClassField,
    fields: &mut HashMap<String, ClassFieldSig>,
    events: &HashMap<String, ClassEventSig>,
    properties: &HashMap<String, ClassPropertySig>,
) -> Result<(), Diagnostic> {
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
            ty: field.ty.clone().unwrap_or(TypeName::Variant),
            array: field.array.clone(),
        },
    );
    Ok(())
}

fn validate_class_field_type(
    field: &crate::ClassField,
    registry: &TypeRegistry,
) -> Result<(), Diagnostic> {
    if let Some(ty) = &field.ty {
        ensure_known_type(ty, registry, field.span)?;
    }
    if field.initializer.is_some() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Class field initializers are not supported yet",
            Some(field.span),
        ));
    }
    Ok(())
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
            let fields: Vec<&crate::ClassField> = match member {
                ClassMember::Field(field) => vec![field],
                ClassMember::Fields(fields) => fields.iter().collect(),
                ClassMember::Const(_) => continue,
                _ => continue,
            };
            for field in fields {
                let owner_field_sig = class_sig
                    .fields
                    .get(&key(&field.name))
                    .expect("field collected");
                if !owner_field_sig.with_events {
                    continue;
                }
                let TypeName::User(source_class_name) = &owner_field_sig.ty else {
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
        if let Some(ty) = &var.ty {
            ensure_known_type(ty, types, var.span)?;
        }
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

    for declare in &program.declares {
        validate_parameter_list(&declare.params, types)?;
        if let Some(return_type) = &declare.return_type {
            ensure_known_type(return_type, types, declare.span)?;
        }

        let name_key = key(&declare.name);
        if let Some(existing) = names.insert(name_key.clone(), "Declare") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!(
                    "Name '{}' conflicts with existing {}",
                    declare.name, existing
                ),
                Some(declare.span),
            ));
        }

        let signature = CallableSig {
            visibility: declare.visibility,
            name: declare.name.clone(),
            _is_iterator: false,
            is_declare: true,
            params: params_to_sigs(&declare.params),
            return_type: declare.return_type.clone(),
        };
        match declare.kind {
            crate::DeclareKind::Function => {
                functions.insert(name_key, signature);
            }
            crate::DeclareKind::Sub => {
                subs.insert(name_key, signature);
            }
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
                _is_iterator: false,
                is_declare: false,
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
                _is_iterator: function.is_iterator,
                is_declare: false,
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
        let ty = if let Some(ty) = &var.ty {
            ensure_known_type(ty, types, var.span)?;
            ty.clone()
        } else if let Some(initializer) = &var.initializer {
            validate_expr(initializer, &symbols, types, signatures)?
        } else {
            TypeName::Variant
        };
        if let Some(initializer) = &var.initializer {
            if var.array.is_some() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "Array declarations cannot use an initializer",
                    Some(initializer.span),
                ));
            }
            let source_type = validate_expr(initializer, &symbols, types, signatures)?;
            ensure_assignable_expr(&ty, &source_type, initializer, types, initializer.span)?;
        }
        let name_key = key(&var.name);
        if symbols.contains_key(&name_key) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Module-level name '{}' is already declared", var.name),
                Some(var.span),
            ));
        }
        let var_type = if var.array.is_some() {
            VarType::Array(ty)
        } else {
            VarType::Scalar(ty)
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
        | ExprKind::Index { .. }
        | ExprKind::IIf { .. }
        | ExprKind::NamedArg { .. }
        | ExprKind::TypeOfIs { .. }
        | ExprKind::AddressOf(_)
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
    let mut saw_yield = false;
    validate_statements(
        &function.body,
        &mut symbols,
        types,
        signatures,
        Context::Function {
            return_type: function.return_type.clone(),
            is_iterator: function.is_iterator,
            saw_return: &mut saw_return,
            saw_yield: &mut saw_yield,
        },
        LoopContext::default(),
        false,
    )?;

    if function.is_iterator {
        if !saw_yield {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                format!(
                    "Iterator Function '{}' must contain at least one Yield statement",
                    function.name
                ),
                Some(function.span),
            ));
        }
        for param in &function.params {
            if param.mode == PassingMode::ByRef {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Iterator Function '{}' cannot have ByRef parameters",
                        function.name
                    ),
                    Some(param.span),
                ));
            }
        }
    } else if !saw_return {
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
