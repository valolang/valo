use super::*;

impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut classes = Vec::new();
        let mut procedures = Vec::new();
        let mut functions = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Type => types.push(self.parse_type_decl()?),
                TokenKind::Enum => enums.push(self.parse_enum_decl(Visibility::Public)?),
                TokenKind::Class => classes.push(self.parse_class_decl()?),
                TokenKind::Sub => procedures.push(self.parse_procedure()?),
                TokenKind::Function => functions.push(self.parse_function()?),
                TokenKind::Public | TokenKind::Private => {
                    let visibility = self.parse_optional_visibility();
                    if self.check_simple(&TokenKind::Enum) {
                        enums.push(self.parse_enum_decl(visibility)?);
                    } else {
                        return Err(self.error_here(
                            "Public/Private are only allowed for Enum or inside Class",
                        ));
                    }
                }
                _ => {
                    return Err(
                        self.error_here("Expected 'Type', 'Enum', 'Class', 'Sub', or 'Function'")
                    );
                }
            }
            self.skip_newlines();
        }

        Ok(Program {
            types,
            enums,
            classes,
            procedures,
            functions,
        })
    }
}
