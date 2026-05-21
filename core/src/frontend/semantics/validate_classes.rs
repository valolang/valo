use super::*;

pub(super) fn validate_class(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    let class_consts = class_constant_symbols(class_decl);
    for member in &class_decl.members {
        match member {
            ClassMember::Field(_) | ClassMember::Fields(_) | ClassMember::Const(_) => {}
            ClassMember::Event(_) => {}
            ClassMember::Sub(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                add_module_symbols(&class_consts, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
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
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                symbols.insert(
                    key(&method.function.name),
                    VarType::Scalar(method.function.return_type.clone()),
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
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                symbols.insert(
                    key(&method.function.name),
                    VarType::Scalar(method.function.return_type.clone()),
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
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&property.params, &mut symbols)?;
                match property.kind {
                    PropertyKind::Get => {
                        let return_type = property
                            .return_type
                            .clone()
                            .expect("property get return type");
                        symbols.insert(key(&property.name), VarType::Scalar(return_type.clone()));
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

fn class_constant_symbols(class_decl: &crate::ClassDecl) -> HashMap<String, VarType> {
    class_decl
        .members
        .iter()
        .filter_map(|member| match member {
            ClassMember::Const(const_decl) => Some((
                key(&const_decl.name),
                VarType::Const(const_decl.ty.clone().unwrap_or(TypeName::Variant)),
            )),
            _ => None,
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
            | ClassMember::Iterator(_) => {}
            ClassMember::Sub(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
                symbols.insert(
                    "me".to_string(),
                    VarType::Scalar(TypeName::User(type_decl.name.clone())),
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
                    VarType::Scalar(TypeName::User(type_decl.name.clone())),
                );
                add_parameters(&method.function.params, &mut symbols)?;
                symbols.insert(
                    key(&method.function.name),
                    VarType::Scalar(method.function.return_type.clone()),
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
                    VarType::Scalar(TypeName::User(type_decl.name.clone())),
                );
                add_parameters(&property.params, &mut symbols)?;
                match property.kind {
                    PropertyKind::Get => {
                        let return_type = property
                            .return_type
                            .clone()
                            .expect("property get return type");
                        symbols.insert(key(&property.name), VarType::Scalar(return_type.clone()));
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
