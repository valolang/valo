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
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut module_vars = Vec::new();
        let mut module_consts = Vec::new();
        let mut classes = Vec::new();
        let mut procedures = Vec::new();
        let mut functions = Vec::new();
        let mut is_class_module = false;
        self.skip_newlines();

        if self.check_simple(&TokenKind::Version) {
            self.parse_cls_envelope()?;
            is_class_module = true;
        }

        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Option => {
                    if !imports.is_empty()
                        || !types.is_empty()
                        || !enums.is_empty()
                        || !module_vars.is_empty()
                        || !module_consts.is_empty()
                        || !classes.is_empty()
                        || !procedures.is_empty()
                        || !functions.is_empty()
                    {
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
                        self.expect_statement_end("Expected newline after Option Explicit")?;
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
                        self.expect_statement_end("Expected newline after Option Base")?;
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
                        self.expect_statement_end("Expected newline after Option Compare")?;
                    } else {
                        return Err(self.error_here("Option must be Explicit, Base, or Compare"));
                    }
                }
                TokenKind::Import => imports.push(self.parse_import_decl()?),
                TokenKind::Identifier(name) if name.eq_ignore_ascii_case("Attribute") => {
                    attributes.push(self.parse_attribute_decl()?);
                }
                TokenKind::Type | TokenKind::Structure => {
                    types.push(self.parse_type_decl(Visibility::Public)?)
                }
                TokenKind::Enum => enums.push(self.parse_enum_decl(Visibility::Public)?),
                _ if is_class_module => {
                    let mut class_members = Vec::new();
                    let mut class_attributes = Vec::new();
                    while !self.is_at_end() {
                        if matches!(self.peek_kind(), TokenKind::Identifier(name) if name.eq_ignore_ascii_case("Attribute"))
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
                        name,
                        attributes: class_attributes,
                        members: class_members,
                        span: crate::runtime::Span::new(
                            crate::runtime::SourcePos::new(1, 1),
                            self.previous().span.end,
                        ),
                    });
                }
                TokenKind::Const => {
                    module_consts.push(self.parse_module_const(Visibility::Private)?)
                }
                TokenKind::Dim => {
                    self.expect_simple(TokenKind::Dim, "Expected 'Dim'")?;
                    module_vars.extend(self.parse_module_vars(Visibility::Private)?);
                }
                TokenKind::Class => classes.push(self.parse_class_decl(Visibility::Public)?),
                TokenKind::Sub => procedures.push(self.parse_procedure(Visibility::Public)?),
                TokenKind::Function => functions.push(self.parse_function(Visibility::Public, false)?),
                TokenKind::Iterator => {
                    self.advance(); // consume Iterator
                    if self.check_simple(&TokenKind::Function) {
                        functions.push(self.parse_function(Visibility::Public, true)?);
                    } else {
                        return Err(self.error_here("Expected 'Function' after 'Iterator'"));
                    }
                }
                TokenKind::Identifier(_) => {
                    module_vars.extend(self.parse_module_vars(Visibility::Private)?);
                }
                TokenKind::Public | TokenKind::Private => {
                    let visibility = self.parse_optional_visibility();
                    let is_iterator = self.match_simple(&TokenKind::Iterator);
                    if self.check_simple(&TokenKind::Enum) {
                        if is_iterator {
                            return Err(self.error_here("Iterator is not supported on Enum"));
                        }
                        enums.push(self.parse_enum_decl(visibility)?);
                    } else if self.check_simple(&TokenKind::Const) {
                        if is_iterator {
                            return Err(self.error_here("Iterator is not supported on Const"));
                        }
                        module_consts.push(self.parse_module_const(visibility)?);
                    } else if self.check_simple(&TokenKind::Type)
                        || self.check_simple(&TokenKind::Structure)
                    {
                        if is_iterator {
                            return Err(self.error_here("Iterator is not supported on Type/Structure"));
                        }
                        types.push(self.parse_type_decl(visibility)?);
                    } else if self.check_simple(&TokenKind::Class) {
                        if is_iterator {
                            return Err(self.error_here("Iterator is not supported on Class"));
                        }
                        classes.push(self.parse_class_decl(visibility)?);
                    } else if self.check_simple(&TokenKind::Sub) {
                        if is_iterator {
                            return Err(self.error_here("Iterator is not supported on Sub"));
                        }
                        procedures.push(self.parse_procedure(visibility)?);
                    } else if self.check_simple(&TokenKind::Function) {
                        functions.push(self.parse_function(visibility, is_iterator)?);
                    } else if self.check_simple(&TokenKind::Dim) {
                        if is_iterator {
                            return Err(self.error_here("Iterator is not supported on Dim"));
                        }
                        self.expect_simple(TokenKind::Dim, "Expected 'Dim'")?;
                        module_vars.extend(self.parse_module_vars(visibility)?);
                    } else if matches!(self.peek_kind(), TokenKind::Identifier(_)) {
                        if is_iterator {
                            return Err(self.error_here("Expected 'Function' after 'Iterator'"));
                        }
                        module_vars.extend(self.parse_module_vars(visibility)?);
                    } else {
                        return Err(self.error_here(
                            "Public/Private are only allowed for module declarations or inside Class",
                        ));
                    }
                }
                _ => {
                    return Err(self.error_here("Expected declaration"));
                }
            }
            self.skip_newlines();
        }

        Ok(Program {
            attributes,
            imports,
            option_explicit,
            option_base,
            option_compare,
            types,
            enums,
            module_vars,
            module_consts,
            classes,
            procedures,
            functions,
        })
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
