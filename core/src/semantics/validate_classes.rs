use super::*;

pub(super) fn validate_class(
    class_decl: &crate::ClassDecl,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
) -> Result<(), Diagnostic> {
    for member in &class_decl.members {
        match member {
            ClassMember::Field(_) => {}
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
                let mut saw_return = false;
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
                    VarType::Scalar(TypeName::User(class_decl.name.clone())),
                );
                add_parameters(&property.params, &mut symbols)?;
                match property.kind {
                    PropertyKind::Get => {
                        let return_type = property
                            .return_type
                            .clone()
                            .expect("property get return type");
                        let mut saw_return = false;
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
