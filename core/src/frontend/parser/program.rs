use super::*;

impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut option_explicit = false;
        let mut option_base = 0;
        let mut saw_option_base = false;
        let mut option_compare = OptionCompare::Binary;
        let mut saw_option_compare = false;
        let mut attributes = Vec::new();
        let mut imports = Vec::new();
        let mut namespace = None;
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut module_vars = Vec::new();
        let mut module_consts = Vec::new();
        let mut declares = Vec::new();
        let mut interfaces = Vec::new();
        let mut classes = Vec::new();
        let mut procedures = Vec::new();
        let mut functions = Vec::new();
        let mut properties = Vec::new();
        let mut is_class_module = false;
        let mut saw_declarations = false;
        self.skip_newlines();

        if self.check_simple(&TokenKind::Version) {
            self.parse_cls_envelope()?;
            is_class_module = true;
        }

        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Namespace => {
                    if saw_declarations || namespace.is_some() {
                        return Err(self.error_here(
                            "Namespace must appear once before declarations in this file",
                        ));
                    }
                    namespace = Some(self.parse_namespace_decl()?);
                }
                TokenKind::End if matches!(self.peek_next_kind(), Some(TokenKind::Namespace)) => {
                    self.advance();
                    self.advance();
                    self.expect_statement_end("Expected newline after End Namespace")?;
                    if !self.is_at_end() {
                        self.skip_newlines();
                        if !self.is_at_end() {
                            return Err(self.error_here(
                                "Declarations after End Namespace are not supported yet",
                            ));
                        }
                    }
                    break;
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
                        option_explicit = true;
                    } else if self.match_simple(&TokenKind::Base) {
                        if saw_option_base {
                            return Err(self.error_here("Option Base is already declared"));
                        }
                        let token = self.advance();
                        let TokenKind::Integer(value) = token.kind else {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::OPTION,
                                "Option Base must be 0 or 1",
                                Some(token.span),
                            )
                            .with_primary_label("invalid Option Base value")
                            .with_help("use either 'Option Base 0' or 'Option Base 1'"));
                        };
                        if value != 0 && value != 1 {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::OPTION,
                                "Option Base must be 0 or 1",
                                Some(token.span),
                            )
                            .with_primary_label("invalid Option Base value")
                            .with_help("use either 'Option Base 0' or 'Option Base 1'"));
                        }
                        option_base = value;
                        saw_option_base = true;
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
                        self.expect_identifier_with(
                            "Module",
                            "Expected 'Module' after 'Option Private'",
                        )?;
                    } else {
                        return Err(self.error_here("Option must be Explicit, Base, or Compare"));
                    }
                    self.expect_statement_end("Expected newline after Option statement")?;
                }
                TokenKind::Import => imports.push(self.parse_import_decl()?),
                TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute") => {
                    attributes.push(self.parse_attribute_decl()?);
                }
                _ => {
                    if is_class_module {
                        let mut class_members = Vec::new();
                        let mut class_attributes = Vec::new();
                        while !self.is_at_end() {
                            if matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute"))
                            {
                                let attribute = self.parse_attribute_decl()?;
                                self.apply_class_attribute(&attribute, &mut class_members);
                                class_attributes.push(attribute);
                            } else {
                                class_members.push(self.parse_class_member()?);
                            }
                            self.skip_newlines();
                        }
                        let name = attributes
                            .iter()
                            .find(|attr| {
                                attr.target.is_empty() && attr.name.eq_ignore_ascii_case("VB_Name")
                            })
                            .map(|attr| attr.value.clone())
                            .unwrap_or_else(|| "ClassModule".to_string());
                        classes.push(ClassDecl {
                            visibility: Visibility::Public,
                            inheritance: ClassInheritance::Normal,
                            name,
                            type_params: Vec::new(),
                            base_class: None,
                            implements: Vec::new(),
                            attributes: class_attributes,
                            members: class_members,
                            span: crate::runtime::Span::new(
                                self.file_id,
                                crate::runtime::SourcePos::new(1, 1),
                                self.previous().span.end,
                            ),
                        });
                        break;
                    }

                    saw_declarations = true;
                    let inheritance = self.parse_optional_class_inheritance();
                    let explicit_visibility = self.parse_optional_visibility();
                    let is_iterator = self.match_simple(&TokenKind::Iterator);

                    match self.peek_kind() {
                        TokenKind::Sub => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            if is_iterator {
                                return Err(self.error_here("Iterator is not supported on Sub"));
                            }
                            procedures.push(self.parse_procedure(visibility)?);
                        }
                        TokenKind::Function => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            functions.push(self.parse_function(visibility, is_iterator)?);
                        }
                        TokenKind::Property => {
                            let visibility = explicit_visibility.unwrap_or(Visibility::Public);
                            properties.push(self.parse_property(visibility, false, is_iterator)?);
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
                            classes.push(self.parse_class_decl(visibility, inheritance)?);
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
                }
            }
            self.skip_newlines();
        }

        Ok(Program {
            namespace,
            attributes,
            imports,
            option_explicit,
            option_base,
            option_compare,
            types,
            enums,
            module_vars,
            module_consts,
            declares,
            interfaces,
            classes,
            procedures,
            functions,
            properties,
        })
    }

    fn parse_namespace_decl(&mut self) -> Result<String, Diagnostic> {
        self.expect_simple(TokenKind::Namespace, "Expected 'Namespace'")?;
        let mut namespace = self.expect_identifier("Expected namespace name")?;
        while self.match_simple(&TokenKind::Dot) {
            namespace.push('.');
            namespace.push_str(
                self.expect_identifier("Expected namespace segment after '.'")?
                    .as_str(),
            );
        }
        self.expect_statement_end("Expected newline after Namespace declaration")?;
        Ok(namespace)
    }

    pub(super) fn parse_cls_envelope(&mut self) -> Result<(), Diagnostic> {
        self.expect_simple(TokenKind::Version, "Expected 'VERSION'")?;
        // Skip version number (e.g., 1.0)
        while !self.is_at_end() && !self.check_simple(&TokenKind::Class) {
            self.advance();
        }
        self.expect_simple(TokenKind::Class, "Expected 'CLASS'")?;
        self.expect_statement_end("Expected newline after VERSION")?;

        if self.match_simple(&TokenKind::Begin) {
            self.skip_newlines();
            let mut depth = 1;
            while !self.is_at_end() && depth > 0 {
                if self.match_simple(&TokenKind::Begin) {
                    depth += 1;
                } else if self.match_simple(&TokenKind::End) {
                    depth -= 1;
                } else {
                    self.advance();
                }
            }
            self.expect_statement_end("Expected newline after END")?;
        }
        Ok(())
    }
}
