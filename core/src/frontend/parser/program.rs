use super::*;

impl Parser {
    fn module_property_field(field: ClassField) -> ModuleVarDecl {
        ModuleVarDecl {
            visibility: field.visibility,
            name: field.name,
            ty: field.ty,
            array: field.array,
            as_new: field.as_new,
            new_args: field.new_args,
            collection_initializer: field.collection_initializer,
            initializer: field.initializer,
            span: field.span,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut option_explicit = false;
        let mut option_strict = false;
        let mut option_infer = true;
        let mut saw_option_infer = false;
        let mut option_compare = OptionCompare::Binary;
        let mut saw_option_compare = false;
        let mut imports = Vec::new();
        let mut namespace = None;
        let mut last_namespace = None;
        let mut root_namespace = None;
        let mut namespace_segments = Vec::new();
        let mut namespace_stack = Vec::new();
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut module_vars = Vec::new();
        let mut module_consts = Vec::new();
        let mut declares = Vec::new();
        let mut delegates = Vec::new();
        let mut interfaces = Vec::new();
        let mut classes = Vec::new();
        let mut procedures = Vec::new();
        let mut functions = Vec::new();
        let mut properties = Vec::new();
        let mut owners = std::collections::HashMap::new();
        let mut saw_declarations = false;
        self.skip_newlines();

        if self.check_simple(&TokenKind::Version) {
            return Err(self
                .error_here("Exported class metadata is not supported; use Class ... End Class"));
        }

        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Namespace => {
                    let ns_name = self.parse_namespace_decl()?;
                    let parts: Vec<String> = ns_name
                        .split('.')
                        .map(|segment| segment.to_string())
                        .collect();
                    namespace_stack.push(parts.len());
                    namespace_segments.extend(parts);
                    namespace = Some(namespace_segments.join("."));
                    last_namespace = namespace.clone();
                    if root_namespace.is_none() {
                        root_namespace = namespace.clone();
                    }
                }
                TokenKind::End if matches!(self.peek_next_kind(), Some(TokenKind::Namespace)) => {
                    self.advance();
                    self.advance();
                    self.expect_statement_end("Expected newline after End Namespace")?;
                    let Some(segment_count) = namespace_stack.pop() else {
                        return Err(self.error_here("End Namespace without matching Namespace"));
                    };
                    for _ in 0..segment_count {
                        namespace_segments.pop();
                    }
                    namespace = if namespace_segments.is_empty() {
                        None
                    } else {
                        Some(namespace_segments.join("."))
                    };
                }
                TokenKind::Option => {
                    if saw_declarations {
                        return Err(
                            self.error_here("Option statements must appear before declarations")
                        );
                    }
                    self.expect_simple(TokenKind::Option, "Expected 'Option'")?;
                    if self.match_simple(&TokenKind::Explicit) {
                        if option_explicit {
                            return Err(self.error_here("Option Explicit is already declared"));
                        }
                        option_explicit = self.parse_option_switch(true)?;
                    } else if self.match_identifier("Strict") {
                        if option_strict {
                            return Err(self.error_here("Option Strict is already declared"));
                        }
                        option_strict = self.parse_option_switch(true)?;
                    } else if self.match_identifier("Infer") {
                        if saw_option_infer {
                            return Err(self.error_here("Option Infer is already declared"));
                        }
                        option_infer = self.parse_option_switch(true)?;
                        saw_option_infer = true;
                    } else if self.match_simple(&TokenKind::Base) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::OPTION,
                            "Option Base has been removed; omitted array lower bounds are always zero",
                            Some(self.previous().span),
                        ).with_help("Remove Option Base and use zero-based array indexing"));
                    } else if self.match_simple(&TokenKind::Compare) {
                        if saw_option_compare {
                            return Err(self.error_here("Option Compare is already declared"));
                        }
                        if self.match_simple(&TokenKind::Binary) {
                            option_compare = OptionCompare::Binary;
                        } else if self.match_simple(&TokenKind::Text) {
                            option_compare = OptionCompare::Text;
                        } else {
                            return Err(self.error_here("Option Compare must be Binary or Text"));
                        }
                        saw_option_compare = true;
                    } else if self.match_simple(&TokenKind::Private) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::OPTION,
                            "Option Private Module has been removed; use declaration access modifiers",
                            Some(self.previous().span),
                        ).with_help("Declare members Private or Public and import modules explicitly"));
                    } else {
                        return Err(
                            self.error_here("Option must be Explicit, Strict, Infer, or Compare")
                        );
                    }
                    self.expect_statement_end("Expected newline after Option statement")?;
                }
                TokenKind::Imports => imports.push(self.parse_import_decl()?),
                TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute") => {
                    return Err(self.error_here("Exported Attribute metadata is not supported; use modern declarations and attributes"));
                }
                _ => {
                    saw_declarations = true;
                    let type_start = types.len();
                    let enum_start = enums.len();
                    let interface_start = interfaces.len();
                    let class_start = classes.len();
                    let function_start = functions.len();
                    let procedure_start = procedures.len();
                    let modern_attributes = self.parse_modern_attributes()?;
                    let inheritance = self.parse_optional_class_inheritance();
                    let explicit_visibility = self.parse_optional_visibility();
                    let is_async = self.match_simple(&TokenKind::Async);
                    let is_partial = self.match_simple(&TokenKind::Partial);
                    let is_iterator = self.match_simple(&TokenKind::Iterator);
                    let is_readonly = self.match_simple(&TokenKind::ReadOnly);
                    let is_writeonly = self.match_simple(&TokenKind::WriteOnly);
                    let is_default = self.match_simple(&TokenKind::Default);

                    // `Overloads` says a procedure shares its name deliberately. Valo works
                    // that out from the declarations themselves, so the word is accepted for
                    // VB.NET source compatibility and carries no meaning of its own.
                    self.match_simple(&TokenKind::Overloads);
                    if is_readonly && is_writeonly {
                        return Err(
                            self.error_here("Property cannot be both ReadOnly and WriteOnly")
                        );
                    }
                    if (is_readonly || is_writeonly || is_default)
                        && !self.check_simple(&TokenKind::Property)
                    {
                        return Err(self.error_here(
                            "ReadOnly, WriteOnly and Default require a property declaration here",
                        ));
                    }

                    let is_module = matches!(self.peek_kind(), TokenKind::Module);
                    match self.peek_kind() {
                        TokenKind::Sub => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Sub"));
                            }
                            procedures.push(self.parse_procedure(
                                visibility,
                                modern_attributes,
                                is_async,
                            )?);
                        }
                        TokenKind::Function => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            functions.push(self.parse_function(
                                visibility,
                                is_iterator,
                                modern_attributes,
                                is_async,
                            )?);
                        }
                        TokenKind::Property => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            for member in self.parse_class_property_members(
                                visibility,
                                true,
                                is_default,
                                is_iterator,
                                is_readonly,
                                is_writeonly,
                            )? {
                                match member {
                                    ClassMember::Property(property) => properties.push(property),
                                    ClassMember::Field(field) => {
                                        module_vars.push(Self::module_property_field(field))
                                    }
                                    _ => unreachable!(
                                        "property lowering produces accessors or backing fields"
                                    ),
                                }
                            }
                        }
                        TokenKind::Type | TokenKind::Structure => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(
                                    self.error_here("Iterator is not supported on Type/Structure")
                                );
                            }
                            types.push(self.parse_type_decl(visibility)?);
                        }
                        TokenKind::Enum => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Enum"));
                            }
                            enums.push(self.parse_enum_decl(visibility)?);
                        }
                        TokenKind::Interface => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(
                                    self.error_here("Iterator is not supported on Interface")
                                );
                            }
                            interfaces.push(self.parse_interface_decl(visibility)?);
                        }
                        TokenKind::Declare => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Declare"));
                            }
                            declares.push(self.parse_declare_decl(visibility)?);
                        }
                        TokenKind::Delegate => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            delegates.push(self.parse_delegate_decl(visibility)?);
                        }
                        TokenKind::Const => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Const"));
                            }
                            module_consts.extend(self.parse_module_consts(visibility)?);
                        }
                        TokenKind::Class => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Class"));
                            }
                            let mut c =
                                self.parse_class_decl(visibility, inheritance, is_partial)?;
                            if let Some(ns) = &namespace
                                && !c.name.starts_with(ns)
                            {
                                c.name = format!("{}.{}", ns, c.name);
                            }
                            classes.push(c);
                        }
                        TokenKind::Module => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if visibility != Visibility::Public {
                                return Err(self.error_here(
                                    "Only Public Module declarations are supported at file scope",
                                ));
                            }
                            if inheritance != ClassInheritance::Normal {
                                return Err(self.error_here(
                                    "Module declarations cannot use inheritance modifiers",
                                ));
                            }
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Module"));
                            }
                            self.parse_module_decl(
                                &mut types,
                                &mut enums,
                                &mut module_vars,
                                &mut module_consts,
                                &mut declares,
                                &mut delegates,
                                &mut interfaces,
                                &mut classes,
                                &mut procedures,
                                &mut functions,
                                &mut properties,
                            )?;
                        }
                        TokenKind::Dim => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Private);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Dim"));
                            }
                            self.advance();
                            module_vars.extend(self.parse_module_vars(visibility)?);
                        }
                        TokenKind::Identifier(_, _) => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Private);
                            if is_iterator {
                                return Err(self.error_here("Expected 'Function' after 'Iterator'"));
                            }
                            module_vars.extend(self.parse_module_vars(visibility)?);
                        }
                        _ => {
                            return Err(self.error_here("Expected declaration"));
                        }
                    }
                    let owner = if is_module {
                        let module_name = &classes
                            .last()
                            .expect("parsed Module has a declaration")
                            .name;
                        namespace
                            .as_ref()
                            .map_or_else(|| module_name.clone(), |ns| format!("{ns}.{module_name}"))
                    } else {
                        namespace.clone().unwrap_or_default()
                    };
                    for decl in &types[type_start..] {
                        owners.insert(decl.span, owner.clone());
                    }
                    for decl in &enums[enum_start..] {
                        owners.insert(decl.span, owner.clone());
                    }
                    for decl in &interfaces[interface_start..] {
                        owners.insert(decl.span, owner.clone());
                    }
                    for decl in &classes[class_start..] {
                        owners
                            .entry(decl.span)
                            .or_insert_with(|| namespace.clone().unwrap_or_default());
                    }
                    for decl in &functions[function_start..] {
                        owners.insert(decl.span, owner.clone());
                    }
                    for decl in &procedures[procedure_start..] {
                        owners.insert(decl.span, owner.clone());
                    }
                }
            }
            self.skip_newlines();
        }

        Ok(Program {
            namespace: last_namespace,
            owners,
            imports,
            option_explicit,
            option_strict,
            option_infer,
            option_compare,
            types,
            enums,
            module_vars,
            module_consts,
            declares,
            delegates,
            interfaces,
            classes,
            procedures,
            functions,
            properties,
        })
    }
    /// Reads the `On` or `Off` that may follow a switchable `Option`.
    ///
    /// VB.NET writes `Option Strict On`; VBA writes `Option Explicit` with
    /// nothing after it and means on. Both spellings are accepted, and
    /// `default_on` is what a bare directive means.
    fn parse_option_switch(&mut self, default_on: bool) -> Result<bool, Diagnostic> {
        // `On` is a keyword, from `On Error`; `Off` is an ordinary name.
        if self.match_simple(&TokenKind::On) || self.match_identifier("On") {
            Ok(true)
        } else if self.match_identifier("Off") {
            Ok(false)
        } else {
            Ok(default_on)
        }
    }

    fn parse_namespace_decl(&mut self) -> Result<String, Diagnostic> {
        self.expect_simple(TokenKind::Namespace, "Expected 'Namespace'")?;
        let mut ns_name = self.expect_identifier("Expected namespace name")?;
        while self.match_simple(&TokenKind::Dot) {
            ns_name.push('.');
            ns_name.push_str(
                self.expect_identifier("Expected namespace segment after '.'")?
                    .as_str(),
            );
        }
        self.expect_statement_end("Expected newline after Namespace declaration")?;
        Ok(ns_name)
    }

    #[allow(clippy::too_many_arguments)]
    fn parse_module_decl(
        &mut self,
        types: &mut Vec<TypeDecl>,
        enums: &mut Vec<EnumDecl>,
        module_vars: &mut Vec<ModuleVarDecl>,
        module_consts: &mut Vec<ConstDecl>,
        declares: &mut Vec<DeclareDecl>,
        delegates: &mut Vec<DelegateDecl>,
        interfaces: &mut Vec<InterfaceDecl>,
        classes: &mut Vec<ClassDecl>,
        procedures: &mut Vec<Procedure>,
        functions: &mut Vec<Function>,
        properties: &mut Vec<ClassProperty>,
    ) -> Result<(), Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Module, "Expected 'Module'")?
            .span;
        let name = self.expect_identifier("Expected module name after 'Module'")?;
        self.expect_statement_end("Expected newline after Module declaration")?;

        let mut shared_members = Vec::new();
        self.skip_newlines();
        while !self.is_at_end()
            && !matches!(
                self.peek_kind(),
                TokenKind::End if matches!(self.peek_next_kind(), Some(TokenKind::Module))
            )
        {
            let modern_attributes = self.parse_modern_attributes()?;
            let inheritance = self.parse_optional_class_inheritance();
            let explicit_visibility = self.parse_optional_visibility();
            let is_async = self.match_simple(&TokenKind::Async);
            let is_partial = self.match_simple(&TokenKind::Partial);
            let is_iterator = self.match_simple(&TokenKind::Iterator);
            let is_readonly = self.match_simple(&TokenKind::ReadOnly);
            let is_writeonly = self.match_simple(&TokenKind::WriteOnly);
            let is_default = self.match_simple(&TokenKind::Default);

            // `Overloads` says a procedure shares its name deliberately. Valo works
            // that out from the declarations themselves, so the word is accepted for
            // VB.NET source compatibility and carries no meaning of its own.
            self.match_simple(&TokenKind::Overloads);
            if is_readonly && is_writeonly {
                return Err(self.error_here("Property cannot be both ReadOnly and WriteOnly"));
            }
            if (is_readonly || is_writeonly || is_default)
                && !self.check_simple(&TokenKind::Property)
            {
                return Err(self.error_here(
                    "ReadOnly, WriteOnly and Default require a property declaration here",
                ));
            }

            match self.peek_kind() {
                TokenKind::Sub => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(
                            self.error_here("Sub declarations cannot use inheritance modifiers")
                        );
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Sub"));
                    }
                    let procedure =
                        self.parse_procedure(visibility, modern_attributes, is_async)?;
                    shared_members.push(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared: true,
                        implements: Vec::new(),
                        procedure: procedure.clone(),
                    }));
                    procedures.push(procedure);
                }
                TokenKind::Function => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(self
                            .error_here("Function declarations cannot use inheritance modifiers"));
                    }
                    let function =
                        self.parse_function(visibility, is_iterator, modern_attributes, is_async)?;
                    shared_members.push(ClassMember::Function(ClassFunction {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared: true,
                        implements: Vec::new(),
                        function: function.clone(),
                    }));
                    functions.push(function);
                }
                TokenKind::Property => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(self
                            .error_here("Property declarations cannot use inheritance modifiers"));
                    }
                    for mut member in self.parse_class_property_members(
                        visibility,
                        true,
                        is_default,
                        is_iterator,
                        is_readonly,
                        is_writeonly,
                    )? {
                        match &mut member {
                            ClassMember::Property(property) => {
                                property.is_shared = true;
                                properties.push(property.clone());
                            }
                            ClassMember::Field(_) => {}
                            _ => unreachable!(
                                "property lowering produces accessors or backing fields"
                            ),
                        }
                        shared_members.push(member);
                    }
                }
                TokenKind::Type | TokenKind::Structure => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(
                            self.error_here("Type declarations cannot use inheritance modifiers")
                        );
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Type/Structure"));
                    }
                    types.push(self.parse_type_decl(visibility)?);
                }
                TokenKind::Enum => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(
                            self.error_here("Enum declarations cannot use inheritance modifiers")
                        );
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Enum"));
                    }
                    enums.push(self.parse_enum_decl(visibility)?);
                }
                TokenKind::Interface => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(self.error_here(
                            "Interface declarations cannot use inheritance modifiers",
                        ));
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Interface"));
                    }
                    interfaces.push(self.parse_interface_decl(visibility)?);
                }
                TokenKind::Declare => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(self
                            .error_here("Declare declarations cannot use inheritance modifiers"));
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Declare"));
                    }
                    declares.push(self.parse_declare_decl(visibility)?);
                }
                TokenKind::Delegate => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    delegates.push(self.parse_delegate_decl(visibility)?);
                }
                TokenKind::Const => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if inheritance != ClassInheritance::Normal {
                        return Err(
                            self.error_here("Const declarations cannot use inheritance modifiers")
                        );
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Const"));
                    }
                    module_consts.extend(self.parse_module_consts(visibility)?);
                }
                TokenKind::Class => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Class"));
                    }
                    classes.push(self.parse_class_decl(visibility, inheritance, is_partial)?);
                }
                TokenKind::Module => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                    if visibility != Visibility::Public {
                        return Err(self.error_here(
                            "Only Public Module declarations are supported at file scope",
                        ));
                    }
                    if inheritance != ClassInheritance::Normal {
                        return Err(
                            self.error_here("Module declarations cannot use inheritance modifiers")
                        );
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Module"));
                    }
                    self.parse_module_decl(
                        types,
                        enums,
                        module_vars,
                        module_consts,
                        declares,
                        delegates,
                        interfaces,
                        classes,
                        procedures,
                        functions,
                        properties,
                    )?;
                }
                TokenKind::Namespace => {
                    return Err(
                        self.error_here("Namespace declarations are not supported inside Module")
                    );
                }
                TokenKind::Dim => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Private);
                    if inheritance != ClassInheritance::Normal {
                        return Err(
                            self.error_here("Dim declarations cannot use inheritance modifiers")
                        );
                    }
                    if is_iterator {
                        return Err(self.error_here("Iterator is not supported on Dim"));
                    }
                    self.advance();
                    let vars = self.parse_module_vars(visibility)?;
                    shared_members.extend(vars.iter().map(|var| {
                        ClassMember::Field(ClassField {
                            visibility: var.visibility,
                            is_shared: true,
                            with_events: false,
                            name: var.name.clone(),
                            ty: var.ty.clone(),
                            array: var.array.clone(),
                            as_new: var.as_new,
                            new_args: var.new_args.clone(),
                            collection_initializer: var.collection_initializer.clone(),
                            initializer: var.initializer.clone(),
                            span: var.span,
                        })
                    }));
                    module_vars.extend(vars);
                }
                TokenKind::Identifier(_, _) => {
                    let visibility = explicit_visibility.unwrap_or(Visibility::Private);
                    if inheritance != ClassInheritance::Normal {
                        return Err(self
                            .error_here("Variable declarations cannot use inheritance modifiers"));
                    }
                    if is_iterator {
                        return Err(self.error_here("Expected 'Function' after 'Iterator'"));
                    }
                    let vars = self.parse_module_vars(visibility)?;
                    shared_members.extend(vars.iter().map(|var| {
                        ClassMember::Field(ClassField {
                            visibility: var.visibility,
                            is_shared: true,
                            with_events: false,
                            name: var.name.clone(),
                            ty: var.ty.clone(),
                            array: var.array.clone(),
                            as_new: var.as_new,
                            new_args: var.new_args.clone(),
                            collection_initializer: var.collection_initializer.clone(),
                            initializer: var.initializer.clone(),
                            span: var.span,
                        })
                    }));
                    module_vars.extend(vars);
                }
                _ => return Err(self.error_here("Expected Module member declaration")),
            }
            self.skip_newlines();
        }

        self.expect_simple(TokenKind::End, "Expected 'End Module'")?;
        let end = self
            .expect_simple(TokenKind::Module, "Expected 'Module' after 'End'")?
            .span;
        self.consume_statement_end();
        classes.push(ClassDecl {
            visibility: Visibility::Public,
            inheritance: ClassInheritance::NotInheritable,
            is_partial: false,
            name,
            type_params: Vec::new(),
            generic_constraints: Vec::new(),
            base_class: None,
            implements: Vec::new(),
            members: shared_members,
            span: crate::runtime::Span::new(self.file_id, start.start, end.end),
        });
        Ok(())
    }
}
