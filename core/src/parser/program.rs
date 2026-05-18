use super::*;

impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut option_explicit = false;
        let mut option_base = 0;
        let mut saw_option_base = false;
        let mut option_compare = OptionCompare::Binary;
        let mut saw_option_compare = false;
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut module_vars = Vec::new();
        let mut module_consts = Vec::new();
        let mut classes = Vec::new();
        let mut procedures = Vec::new();
        let mut functions = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Option => {
                    if !types.is_empty()
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
                            return Err(Diagnostic::new(crate::runtime::DiagnosticCode::OPTION, "Option Base must be 0 or 1", Some(token.span),)
                            .with_primary_label("invalid Option Base value")
                            .with_help("use either 'Option Base 0' or 'Option Base 1'"));
                        };
                        if value != 0 && value != 1 {
                            return Err(Diagnostic::new(crate::runtime::DiagnosticCode::OPTION, "Option Base must be 0 or 1", Some(token.span),)
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
                TokenKind::Type => types.push(self.parse_type_decl()?),
                TokenKind::Enum => enums.push(self.parse_enum_decl(Visibility::Public)?),
                TokenKind::Const => {
                    module_consts.push(self.parse_module_const(Visibility::Private)?)
                }
                TokenKind::Class => classes.push(self.parse_class_decl()?),
                TokenKind::Sub => procedures.push(self.parse_procedure()?),
                TokenKind::Function => functions.push(self.parse_function()?),
                TokenKind::Identifier(_) => {
                    module_vars.push(self.parse_module_var(Visibility::Private)?);
                }
                TokenKind::Public | TokenKind::Private => {
                    let visibility = self.parse_optional_visibility();
                    if self.check_simple(&TokenKind::Enum) {
                        enums.push(self.parse_enum_decl(visibility)?);
                    } else if self.check_simple(&TokenKind::Const) {
                        module_consts.push(self.parse_module_const(visibility)?);
                    } else if matches!(self.peek_kind(), TokenKind::Identifier(_)) {
                        module_vars.push(self.parse_module_var(visibility)?);
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
}
