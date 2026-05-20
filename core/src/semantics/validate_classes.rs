use super::*;

pub(super) fn validate_class(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    for member in &class_decl.members {
        match member {
            ClassMember::Field(_) | ClassMember::Fields(_) => {}
            ClassMember::Event(_) => {}
            ClassMember::Sub(method) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
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
                validate_statements(
                    &method.function.body,
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: class_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        saw_return: &mut saw_return,
                    },
                    LoopContext::default(),
                    false,
                )?;
                if !saw_return {
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
                validate_statements(
                    &method.function.body,
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: class_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        saw_return: &mut saw_return,
                    },
                    LoopContext::default(),
                    false,
                )?;
                if !saw_return {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!("Iterator '{}' must return a value", method.function.name),
                        Some(method.function.span),
                    ));
                }
            }
            ClassMember::Property(property) => {
                let mut symbols = HashMap::new();
                add_module_symbols(module_symbols, &mut symbols);
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
                        validate_statements(
                            &property.body,
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyGet {
                                class_name: class_decl.name.clone(),
                                return_type,
                                saw_return: &mut saw_return,
                            },
                            LoopContext::default(),
                            false,
                        )?;
                        if !saw_return {
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
                validate_statements(
                    &method.function.body,
                    &mut symbols,
                    types,
                    signatures,
                    Context::MethodFunction {
                        class_name: type_decl.name.clone(),
                        return_type: method.function.return_type.clone(),
                        saw_return: &mut saw_return,
                    },
                    LoopContext::default(),
                    false,
                )?;
                if !saw_return {
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
                        validate_statements(
                            &property.body,
                            &mut symbols,
                            types,
                            signatures,
                            Context::PropertyGet {
                                class_name: type_decl.name.clone(),
                                return_type,
                                saw_return: &mut saw_return,
                            },
                            LoopContext::default(),
                            false,
                        )?;
                        if !saw_return {
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
