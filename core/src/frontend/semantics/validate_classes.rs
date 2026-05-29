use super::*;
use crate::runtime::Span;

pub(super) fn validate_class(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
    option_explicit: bool,
) -> Result<(), Diagnostic> {
    validate_implements_common(
        &class_decl.name,
        &class_decl.implements,
        &class_decl.members,
        class_decl.span,
        types,
    )?;
    let class_consts = class_constant_symbols(class_decl);
    for member in &class_decl.members {
        match member {
            ClassMember::Field(_)
            | ClassMember::Fields(_)
            | ClassMember::Const(_)
            | ClassMember::Type(_)
            | ClassMember::Declare(_)
            | ClassMember::Enum(_)
            | ClassMember::Class(_) => {}
            ClassMember::Operator(op) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                add_module_symbols(&class_consts, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&op.params, &mut symbols)?;
                let mut saw_return = false;
                let mut saw_yield = false;
                validate_statements(
                    &op.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::Function {
                            return_type: op.return_type.clone(),
                            return_slot: None,
                            is_iterator: false,
                            saw_return: &mut saw_return,
                            saw_yield: &mut saw_yield,
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
            }
            ClassMember::Event(_) => {}
            ClassMember::Sub(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                add_module_symbols(&class_consts, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.procedure.params, &mut symbols)?;
                validate_statements(
                    &method.procedure.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::MethodSub {
                            class_name: class_decl.name.clone(),
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
            }
            ClassMember::Function(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                add_module_symbols(&class_consts, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                symbols.insert(
                    key(&method.function.name),
                    VarType::FunctionReturn(method.function.return_type.clone()),
                );
                let mut saw_return = assigns_to_name(&method.function.body, &method.function.name);
                let mut saw_yield = false;
                validate_statements(
                    &method.function.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::MethodFunction {
                            class_name: class_decl.name.clone(),
                            return_type: method.function.return_type.clone(),
                            return_slot: Some(format!("__return_{}", method.function.name)),
                            is_iterator: method.function.is_iterator,
                            saw_return: &mut saw_return,
                            saw_yield: &mut saw_yield,
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
                if method.function.is_iterator {
                    if !saw_yield {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            format!(
                                "Iterator Function '{}' must contain at least one Yield statement",
                                method.function.name
                            ),
                            Some(method.function.span),
                        ));
                    }
                    for param in &method.function.params {
                        if param.mode == PassingMode::ByRef {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                format!(
                                    "Iterator Function '{}' cannot have ByRef parameters",
                                    method.function.name
                                ),
                                Some(param.span),
                            ));
                        }
                    }
                } else if !saw_return {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!("Function '{}' must return a value", method.function.name),
                        Some(method.function.span),
                    ));
                }
            }
            ClassMember::Iterator(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                add_module_symbols(&class_consts, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                symbols.insert(
                    key(&method.function.name),
                    VarType::FunctionReturn(method.function.return_type.clone()),
                );
                let mut saw_return = assigns_to_name(&method.function.body, &method.function.name);
                let mut saw_yield = false;
                validate_statements(
                    &method.function.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::MethodFunction {
                            class_name: class_decl.name.clone(),
                            return_type: method.function.return_type.clone(),
                            return_slot: Some(format!("__return_{}", method.function.name)),
                            is_iterator: true,
                            saw_return: &mut saw_return,
                            saw_yield: &mut saw_yield,
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
                if !saw_yield && !saw_return {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!(
                            "Iterator '{}' must return a value or yield values",
                            method.function.name
                        ),
                        Some(method.function.span),
                    ));
                }
            }
            ClassMember::Property(property) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                add_module_symbols(&class_consts, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&property.params, &mut symbols)?;
                match property.kind {
                    PropertyKind::Get => {
                        let return_type = property
                            .return_type
                            .clone()
                            .expect("property get return type");
                        symbols.insert(
                            key(&property.name),
                            VarType::FunctionReturn(return_type.clone()),
                        );
                        let mut saw_return = assigns_to_name(&property.body, &property.name);
                        let mut saw_yield = false;
                        validate_statements(
                            &property.body,
                            &mut StmtValidation {
                                symbols: &mut symbols,
                                types,
                                signatures,
                                context: &mut Context::PropertyGet {
                                    class_name: class_decl.name.clone(),
                                    return_type,
                                    return_slot: Some(format!("__return_{}", property.name)),
                                    is_iterator: property.is_iterator,
                                    saw_return: &mut saw_return,
                                    saw_yield: &mut saw_yield,
                                },
                                loop_context: LoopContext::default(),
                                in_with: false,
                                option_explicit,
                            },
                        )?;
                        if property.is_iterator {
                            if !saw_yield {
                                return Err(Diagnostic::new(
                                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                                    format!(
                                        "Iterator Property '{}' must contain at least one Yield statement",
                                        property.name
                                    ),
                                    Some(property.span),
                                ));
                            }
                            for param in &property.params {
                                if param.mode == PassingMode::ByRef {
                                    return Err(Diagnostic::new(
                                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                        format!(
                                            "Iterator Property '{}' cannot have ByRef parameters",
                                            property.name
                                        ),
                                        Some(param.span),
                                    ));
                                }
                            }
                        } else if !saw_return {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!("Property Get '{}' must return a value", property.name),
                                Some(property.span),
                            ));
                        }
                    }
                    PropertyKind::Let | PropertyKind::Set => {
                        validate_statements(
                            &property.body,
                            &mut StmtValidation {
                                symbols: &mut symbols,
                                types,
                                signatures,
                                context: &mut Context::PropertyLetSet {
                                    class_name: class_decl.name.clone(),
                                },
                                loop_context: LoopContext::default(),
                                in_with: false,
                                option_explicit,
                            },
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_implements_common(
    decl_name: &str,
    implements: &[TypeName],
    members: &[ClassMember],
    span: Span,
    types: &TypeRegistry,
) -> Result<(), Diagnostic> {
    let mut implemented = std::collections::HashSet::new();
    for interface_ty in implements {
        let interface_ty = types.canonical_type_name(interface_ty);
        let (interface_name, bindings) = match interface_ty {
            TypeName::User(name) => (name.clone(), Vec::new()),
            TypeName::GenericInstance { name, args } => {
                let interface = types.get_interface(&name).ok_or_else(|| {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Interface '{}' is not defined", name),
                        Some(span),
                    )
                })?;
                let bindings: Vec<(String, TypeName)> = interface
                    .type_params
                    .iter()
                    .cloned()
                    .zip(args.iter().cloned())
                    .collect();
                (name, bindings)
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Implements target must be an Interface",
                    Some(span),
                ));
            }
        };
        let interface = types.get_interface(&interface_name).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Interface '{}' is not defined", interface_name),
                Some(span),
            )
        })?;
        for method in interface.subs.values() {
            let target = find_explicit_sub_impl(
                members,
                decl_name,
                span,
                &interface_name,
                &method.name,
                &bindings,
            )?;
            let method = method.substitute_generics(&bindings);
            if !signature_matches(&target.params, None, &method.params, None) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Implementation '{}.{}' signature does not match '{}.{}'",
                        decl_name, target.name, interface.name, method.name
                    ),
                    Some(span),
                ));
            }
            let key = format!("{}.{}", key(&interface_name), key(&method.name));
            if !implemented.insert(key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!(
                        "Interface member '{}.{}' is implemented more than once",
                        interface.name, method.name
                    ),
                    Some(span),
                ));
            }
        }
        for method in interface.functions.values() {
            let target = find_explicit_function_impl(
                members,
                decl_name,
                span,
                &interface_name,
                &method.name,
                &bindings,
            )?;
            let method = method.substitute_generics(&bindings);
            if !signature_matches(
                &target.params,
                target.return_type.as_ref(),
                &method.params,
                method.return_type.as_ref(),
            ) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Implementation '{}.{}' signature does not match '{}.{}'",
                        decl_name, target.name, interface.name, method.name
                    ),
                    Some(span),
                ));
            }
            let key = format!("{}.{}", key(&interface_name), key(&method.name));
            if !implemented.insert(key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!(
                        "Interface member '{}.{}' is implemented more than once",
                        interface.name, method.name
                    ),
                    Some(span),
                ));
            }
        }
        for property in interface.properties.values() {
            let property_bound = property.substitute_generics(&bindings);
            if let Some(get) = &property_bound.get {
                let target = find_explicit_property_impl(
                    members,
                    decl_name,
                    span,
                    &interface_name,
                    &property.name,
                    PropertyKind::Get,
                    &bindings,
                )?;
                if !signature_matches(
                    &target.params,
                    target.return_type.as_ref(),
                    &get.params,
                    get.return_type.as_ref(),
                ) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Implementation '{}.{}' signature does not match '{}.{}'",
                            decl_name, target.name, interface.name, property.name
                        ),
                        Some(span),
                    ));
                }
            }
            if let Some(let_) = &property_bound.let_ {
                let target = find_explicit_property_impl(
                    members,
                    decl_name,
                    span,
                    &interface_name,
                    &property.name,
                    PropertyKind::Let,
                    &bindings,
                )?;
                if !signature_matches(&target.params, None, &let_.params, None) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Implementation '{}.{}' signature does not match '{}.{}'",
                            decl_name, target.name, interface.name, property.name
                        ),
                        Some(span),
                    ));
                }
            }
            if let Some(set) = &property_bound.set {
                let target = find_explicit_property_impl(
                    members,
                    decl_name,
                    span,
                    &interface_name,
                    &property.name,
                    PropertyKind::Set,
                    &bindings,
                )?;
                if !signature_matches(&target.params, None, &set.params, None) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Implementation '{}.{}' signature does not match '{}.{}'",
                            decl_name, target.name, interface.name, property.name
                        ),
                        Some(span),
                    ));
                }
            }
        }
        for event in interface.events.values() {
            let event = event.substitute_generics(&bindings);
            let has_event = if let Some(class_sig) = types.get_class(decl_name) {
                class_sig
                    .events
                    .get(&key(&event.name))
                    .is_some_and(|candidate| {
                        signature_matches(&candidate.params, None, &event.params, None)
                    })
            } else if let Some(type_sig) = types.get(decl_name) {
                // Structures don't support events yet, but check for completeness
                _ = type_sig;
                false
            } else {
                false
            };

            if !has_event {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Type '{}' is missing implementation for event '{}.{}'",
                        decl_name, interface.name, event.name
                    ),
                    Some(span),
                ));
            }
        }
    }
    Ok(())
}

fn find_explicit_sub_impl(
    members: &[ClassMember],
    decl_name: &str,
    span: Span,
    interface_name: &str,
    member_name: &str,
    bindings: &[(String, TypeName)],
) -> Result<CallableSig, Diagnostic> {
    for member in members {
        if let ClassMember::Sub(method) = member
            && method
                .implements
                .iter()
                .any(|clause| implements_matches(clause, interface_name, member_name, bindings))
        {
            return Ok(CallableSig {
                attributes: Vec::new(),
                visibility: method.visibility,
                name: method.procedure.name.clone(),
                type_params: method.procedure.type_params.clone(),
                generic_constraints: method.procedure.generic_constraints.clone(),
                is_shared: method.is_shared,
                _is_iterator: false,
                is_declare: false,
                params: params_to_sigs(&method.procedure.params),
                return_type: None,
            });
        }
    }
    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Type '{}' is missing implementation for '{}.{}'",
            decl_name, interface_name, member_name
        ),
        Some(span),
    ))
}

fn find_explicit_function_impl(
    members: &[ClassMember],
    decl_name: &str,
    span: Span,
    interface_name: &str,
    member_name: &str,
    bindings: &[(String, TypeName)],
) -> Result<CallableSig, Diagnostic> {
    for member in members {
        if let ClassMember::Function(method) = member
            && method
                .implements
                .iter()
                .any(|clause| implements_matches(clause, interface_name, member_name, bindings))
        {
            return Ok(CallableSig {
                attributes: Vec::new(),
                visibility: method.visibility,
                name: method.function.name.clone(),
                type_params: method.function.type_params.clone(),
                generic_constraints: method.function.generic_constraints.clone(),
                is_shared: method.is_shared,
                _is_iterator: method.function.is_iterator,
                is_declare: false,
                params: params_to_sigs(&method.function.params),
                return_type: Some(method.function.return_type.clone()),
            });
        }
    }
    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Type '{}' is missing implementation for '{}.{}'",
            decl_name, interface_name, member_name
        ),
        Some(span),
    ))
}

fn find_explicit_property_impl(
    members: &[ClassMember],
    decl_name: &str,
    span: Span,
    interface_name: &str,
    member_name: &str,
    kind: PropertyKind,
    bindings: &[(String, TypeName)],
) -> Result<CallableSig, Diagnostic> {
    for member in members {
        if let ClassMember::Property(property) = member
            && property.kind == kind
            && property
                .implements
                .iter()
                .any(|clause| implements_matches(clause, interface_name, member_name, bindings))
        {
            return Ok(CallableSig {
                attributes: Vec::new(),
                visibility: property.visibility,
                name: property.name.clone(),
                type_params: Vec::new(),
                generic_constraints: Vec::new(),
                is_shared: property.is_shared,
                _is_iterator: property.is_iterator,
                is_declare: false,
                params: params_to_sigs(&property.params),
                return_type: property.return_type.clone(),
            });
        }
    }
    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
        format!(
            "Type '{}' is missing implementation for '{}.{}'",
            decl_name, interface_name, member_name
        ),
        Some(span),
    ))
}

fn implements_matches(
    clause: &crate::ImplementsClause,
    interface_name: &str,
    member_name: &str,
    bindings: &[(String, TypeName)],
) -> bool {
    let target_ty = clause.interface_name.substitute_generics(bindings);
    let target_name = match &target_ty {
        TypeName::User(name) => name,
        TypeName::GenericInstance { name, .. } => name,
        _ => return false,
    };
    target_name.eq_ignore_ascii_case(interface_name)
        && clause.member_name.eq_ignore_ascii_case(member_name)
}

fn signature_matches(
    actual_params: &[ParamSig],
    actual_return: Option<&TypeName>,
    expected_params: &[ParamSig],
    expected_return: Option<&TypeName>,
) -> bool {
    actual_return
        .zip(expected_return)
        .is_none_or(|(left, right)| left.same_type(right))
        && actual_return.is_some() == expected_return.is_some()
        && actual_params.len() == expected_params.len()
        && actual_params
            .iter()
            .zip(expected_params.iter())
            .all(|(left, right)| {
                left.mode == right.mode
                    && left.ty.same_type(&right.ty)
                    && left.is_optional == right.is_optional
                    && left.is_param_array == right.is_param_array
            })
}

fn class_constant_symbols(class_decl: &crate::ClassDecl) -> HashMap<String, VarType> {
    class_decl
        .members
        .iter()
        .filter_map(|member| match member {
            ClassMember::Const(const_decl) => Some((
                key(&const_decl.name),
                VarType::Const(
                    Visibility::Public,
                    const_decl.ty.clone().unwrap_or(TypeName::Variant),
                ),
            )),
            ClassMember::Field(_)
            | ClassMember::Fields(_)
            | ClassMember::Event(_)
            | ClassMember::Sub(_)
            | ClassMember::Function(_)
            | ClassMember::Iterator(_)
            | ClassMember::Property(_)
            | ClassMember::Type(_)
            | ClassMember::Declare(_)
            | ClassMember::Enum(_)
            | ClassMember::Operator(_)
            | ClassMember::Class(_) => None,
        })
        .collect()
}

pub(super) fn validate_structure(
    type_decl: &crate::TypeDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
    option_explicit: bool,
) -> Result<(), Diagnostic> {
    validate_implements_common(
        &type_decl.name,
        &type_decl.implements,
        &type_decl.members,
        type_decl.span,
        types,
    )?;
    for member in &type_decl.members {
        match member {
            ClassMember::Field(_)
            | ClassMember::Fields(_)
            | ClassMember::Const(_)
            | ClassMember::Event(_)
            | ClassMember::Iterator(_)
            | ClassMember::Type(_)
            | ClassMember::Declare(_)
            | ClassMember::Enum(_)
            | ClassMember::Class(_) => {}
            ClassMember::Operator(op) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(type_decl.name.clone())),
                );
                add_parameters(&op.params, &mut symbols)?;
                let mut saw_return = false;
                let mut saw_yield = false;
                validate_statements(
                    &op.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::Function {
                            return_type: op.return_type.clone(),
                            return_slot: None,
                            is_iterator: false,
                            saw_return: &mut saw_return,
                            saw_yield: &mut saw_yield,
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
            }
            ClassMember::Sub(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(type_decl.name.clone())),
                );
                add_parameters(&method.procedure.params, &mut symbols)?;
                validate_statements(
                    &method.procedure.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::MethodSub {
                            class_name: type_decl.name.clone(),
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
            }
            ClassMember::Function(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(type_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                symbols.insert(
                    key(&method.function.name),
                    VarType::FunctionReturn(method.function.return_type.clone()),
                );
                let mut saw_return = assigns_to_name(&method.function.body, &method.function.name);
                let mut saw_yield = false;
                validate_statements(
                    &method.function.body,
                    &mut StmtValidation {
                        symbols: &mut symbols,
                        types,
                        signatures,
                        context: &mut Context::MethodFunction {
                            class_name: type_decl.name.clone(),
                            return_type: method.function.return_type.clone(),
                            return_slot: Some(format!("__return_{}", method.function.name)),
                            is_iterator: method.function.is_iterator,
                            saw_return: &mut saw_return,
                            saw_yield: &mut saw_yield,
                        },
                        loop_context: LoopContext::default(),
                        in_with: false,
                        option_explicit,
                    },
                )?;
                if method.function.is_iterator {
                    if !saw_yield {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            format!(
                                "Iterator Function '{}' must contain at least one Yield statement",
                                method.function.name
                            ),
                            Some(method.function.span),
                        ));
                    }
                    for param in &method.function.params {
                        if param.mode == PassingMode::ByRef {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                format!(
                                    "Iterator Function '{}' cannot have ByRef parameters",
                                    method.function.name
                                ),
                                Some(param.span),
                            ));
                        }
                    }
                } else if !saw_return {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!("Function '{}' must return a value", method.function.name),
                        Some(method.function.span),
                    ));
                }
            }
            ClassMember::Property(property) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(Visibility::Public, TypeName::User(type_decl.name.clone())),
                );
                add_parameters(&property.params, &mut symbols)?;
                match property.kind {
                    PropertyKind::Get => {
                        let return_type = property
                            .return_type
                            .clone()
                            .expect("property get return type");
                        symbols.insert(
                            key(&property.name),
                            VarType::FunctionReturn(return_type.clone()),
                        );
                        let mut saw_return = assigns_to_name(&property.body, &property.name);
                        let mut saw_yield = false;
                        validate_statements(
                            &property.body,
                            &mut StmtValidation {
                                symbols: &mut symbols,
                                types,
                                signatures,
                                context: &mut Context::PropertyGet {
                                    class_name: type_decl.name.clone(),
                                    return_type,
                                    return_slot: Some(format!("__return_{}", property.name)),
                                    is_iterator: property.is_iterator,
                                    saw_return: &mut saw_return,
                                    saw_yield: &mut saw_yield,
                                },
                                loop_context: LoopContext::default(),
                                in_with: false,
                                option_explicit,
                            },
                        )?;
                        if property.is_iterator {
                            if !saw_yield {
                                return Err(Diagnostic::new(
                                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                                    format!(
                                        "Iterator Property '{}' must contain at least one Yield statement",
                                        property.name
                                    ),
                                    Some(property.span),
                                ));
                            }
                            for param in &property.params {
                                if param.mode == PassingMode::ByRef {
                                    return Err(Diagnostic::new(
                                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                        format!(
                                            "Iterator Property '{}' cannot have ByRef parameters",
                                            property.name
                                        ),
                                        Some(param.span),
                                    ));
                                }
                            }
                        } else if !saw_return {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!("Property Get '{}' must return a value", property.name),
                                Some(property.span),
                            ));
                        }
                    }
                    PropertyKind::Let | PropertyKind::Set => {
                        validate_statements(
                            &property.body,
                            &mut StmtValidation {
                                symbols: &mut symbols,
                                types,
                                signatures,
                                context: &mut Context::PropertyLetSet {
                                    class_name: type_decl.name.clone(),
                                },
                                loop_context: LoopContext::default(),
                                in_with: false,
                                option_explicit,
                            },
                        )?;
                    }
                }
            }
        }
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
