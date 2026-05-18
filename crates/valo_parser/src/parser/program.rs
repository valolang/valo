use super::*;

impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut types = Vec::new();
        let mut classes = Vec::new();
        let mut procedures = Vec::new();
        let mut functions = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Type => types.push(self.parse_type_decl()?),
                TokenKind::Class => classes.push(self.parse_class_decl()?),
                TokenKind::Sub => procedures.push(self.parse_procedure()?),
                TokenKind::Function => functions.push(self.parse_function()?),
                TokenKind::Public | TokenKind::Private => {
                    return Err(self.error_here("Public/Private are only allowed inside Class"));
                }
                _ => return Err(self.error_here("Expected 'Type', 'Class', 'Sub', or 'Function'")),
            }
            self.skip_newlines();
        }

        Ok(Program {
            types,
            classes,
            procedures,
            functions,
        })
    }
}
