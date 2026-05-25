use super::*;

pub(super) fn validate_class(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    validate_implements(class_decl, types)?;
    let class_consts = class_constant_symbols(class_decl);
    for member in &class_decl.members {
        match member {
            ClassMember::Field(_)
            | ClassMember::Fields(_)
            | ClassMember::Const(_)
            | ClassMember::Type(_)
            | ClassMember::Declare(_)
            | ClassMember::Enum(_) => {}
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
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodSub {
                        class_name: class_decl.name.clone(),
                    },
                    LoopContext::default(),
                    false,
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
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: class_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        return_slot: Some(format!("__return_{}", method.function.name)),
                        is_iterator: method.function.is_iterator,
                        saw_return: &mut saw_return,
                        saw_yield: &mut saw_yield,
                    },
                    LoopContext::default(),
                    false,
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
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: class_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        return_slot: Some(format!("__return_{}", method.function.name)),
                        is_iterator: true,
                        saw_return: &mut saw_return,
                        saw_yield: &mut saw_yield,
                    },
                    LoopContext::default(),
                    false,
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
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyGet {
                                class_name: class_decl.name.clone(),
                                return_type,
                                return_slot: Some(format!("__return_{}", property.name)),
                                is_iterator: property.is_iterator,
                                saw_return: &mut saw_return,
                                saw_yield: &mut saw_yield,
                            },
                            LoopContext::default(),
                            false,
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
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyLetSet {
                                class_name: class_decl.name.clone(),
                            },
                            LoopContext::default(),
                            false,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_implements(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
) -> Result<(), Diagnostic> {
    let class_sig = types
        .get_class(&class_decl.name)
        .expect("class signature collected");
    let mut implemented = std::collections::HashSet::new();
    for interface_name in &class_decl.implements {
        let TypeName::User(interface_name) = interface_name else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Implements target must be an Interface",
                Some(class_decl.span),
            ));
        };
        let interface = types.get_interface(interface_name).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Interface '{}' is not defined", interface_name),
                Some(class_decl.span),
            )
        })?;
        for method in interface.subs.values() {
            let target = find_explicit_sub_impl(class_decl, interface_name, &method.name)?;
            if !signature_matches(&target.params, None, &method.params, None) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Implementation '{}.{}' signature does not match '{}.{}'",
                        class_decl.name, target.name, interface.name, method.name
                    ),
                    Some(class_decl.span),
                ));
            }
            let key = format!("{}.{}", key(interface_name), key(&method.name));
            if !implemented.insert(key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!(
                        "Interface member '{}.{}' is implemented more than once",
                        interface.name, method.name
                    ),
                    Some(class_decl.span),
                ));
            }
        }
        for method in interface.functions.values() {
            let target = find_explicit_function_impl(class_decl, interface_name, &method.name)?;
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
                        class_decl.name, target.name, interface.name, method.name
                    ),
                    Some(class_decl.span),
                ));
            }
            let key = format!("{}.{}", key(interface_name), key(&method.name));
            if !implemented.insert(key) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!(
                        "Interface member '{}.{}' is implemented more than once",
                        interface.name, method.name
                    ),
                    Some(class_decl.span),
                ));
            }
        }
        for property in interface.properties.values() {
            if let Some(get) = &property.get {
                let target = find_explicit_property_impl(
                    class_decl,
                    interface_name,
                    &property.name,
                    PropertyKind::Get,
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
                            class_decl.name, target.name, interface.name, property.name
                        ),
                        Some(class_decl.span),
                    ));
                }
            }
            if let Some(let_) = &property.let_ {
                let target = find_explicit_property_impl(
                    class_decl,
                    interface_name,
                    &property.name,
                    PropertyKind::Let,
                )?;
                if !signature_matches(&target.params, None, &let_.params, None) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Implementation '{}.{}' signature does not match '{}.{}'",
                            class_decl.name, target.name, interface.name, property.name
                        ),
                        Some(class_decl.span),
                    ));
                }
            }
            if let Some(set) = &property.set {
                let target = find_explicit_property_impl(
                    class_decl,
                    interface_name,
                    &property.name,
                    PropertyKind::Set,
                )?;
                if !signature_matches(&target.params, None, &set.params, None) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Implementation '{}.{}' signature does not match '{}.{}'",
                            class_decl.name, target.name, interface.name, property.name
                        ),
                        Some(class_decl.span),
                    ));
                }
            }
        }
        for event in interface.events.values() {
            if !class_sig
                .events
                .get(&key(&event.name))
                .is_some_and(|candidate| {
                    signature_matches(&candidate.params, None, &event.params, None)
                })
            {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!(
                        "Class '{}' is missing implementation for event '{}.{}'",
                        class_decl.name, interface.name, event.name
                    ),
                    Some(class_decl.span),
                ));
            }
        }
    }
    Ok(())
}

fn find_explicit_sub_impl(
    class_decl: &crate::ClassDecl,
    interface_name: &str,
    member_name: &str,
) -> Result<CallableSig, Diagnostic> {
    for member in &class_decl.members {
        if let ClassMember::Sub(method) = member
            && method
                .implements
                .iter()
                .any(|clause| implements_matches(clause, interface_name, member_name))
        {
            return Ok(CallableSig {
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
            "Class '{}' is missing implementation for '{}.{}'",
            class_decl.name, interface_name, member_name
        ),
        Some(class_decl.span),
    ))
}

fn find_explicit_function_impl(
    class_decl: &crate::ClassDecl,
    interface_name: &str,
    member_name: &str,
) -> Result<CallableSig, Diagnostic> {
    for member in &class_decl.members {
        if let ClassMember::Function(method) = member
            && method
                .implements
                .iter()
                .any(|clause| implements_matches(clause, interface_name, member_name))
        {
            return Ok(CallableSig {
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
            "Class '{}' is missing implementation for '{}.{}'",
            class_decl.name, interface_name, member_name
        ),
        Some(class_decl.span),
    ))
}

fn find_explicit_property_impl(
    class_decl: &crate::ClassDecl,
    interface_name: &str,
    member_name: &str,
    kind: PropertyKind,
) -> Result<CallableSig, Diagnostic> {
    for member in &class_decl.members {
        if let ClassMember::Property(property) = member
            && property.kind == kind
            && property
                .implements
                .iter()
                .any(|clause| implements_matches(clause, interface_name, member_name))
        {
            return Ok(CallableSig {
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
            "Class '{}' is missing implementation for '{}.{}'",
            class_decl.name, interface_name, member_name
        ),
        Some(class_decl.span),
    ))
}

fn implements_matches(
    clause: &crate::ImplementsClause,
    interface_name: &str,
    member_name: &str,
) -> bool {
    matches!(&clause.interface_name, TypeName::User(name) if name.eq_ignore_ascii_case(interface_name))
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
            | ClassMember::Enum(_) => None,
        })
        .collect()
}

pub(super) fn validate_structure(
    type_decl: &crate::TypeDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    for member in &type_decl.members {
        match member {
            ClassMember::Field(_)
            | ClassMember::Fields(_)
            | ClassMember::Const(_)
            | ClassMember::Event(_)
            | ClassMember::Iterator(_)
            | ClassMember::Type(_)
            | ClassMember::Declare(_)
            | ClassMember::Enum(_) => {}
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
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodSub {
                        class_name: type_decl.name.clone(),
                    },
                    LoopContext::default(),
                    false,
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
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: type_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        return_slot: Some(format!("__return_{}", method.function.name)),
                        is_iterator: method.function.is_iterator,
                        saw_return: &mut saw_return,
                        saw_yield: &mut saw_yield,
                    },
                    LoopContext::default(),
                    false,
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
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyGet {
                                class_name: type_decl.name.clone(),
                                return_type,
                                return_slot: Some(format!("__return_{}", property.name)),
                                is_iterator: property.is_iterator,
                                saw_return: &mut saw_return,
                                saw_yield: &mut saw_yield,
                            },
                            LoopContext::default(),
                            false,
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
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyLetSet {
                                class_name: type_decl.name.clone(),
                            },
                            LoopContext::default(),
                            false,
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
