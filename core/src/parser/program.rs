use super::*;

impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut option_explicit = false;
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
                    } else {
                        return Err(self.error_here("Only Option Explicit is supported; other Option statements are not implemented"));
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
