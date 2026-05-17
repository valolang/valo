use super::*;
use valo_runtime::{Diagnostic, Span, TypeName};

impl Parser {
    pub(super) fn parse_type_decl(&mut self) -> Result<TypeDecl, Diagnostic> {
        let start = self.expect_simple(TokenKind::Type, "Expected 'Type'")?.span;
        let name = self.expect_identifier("Expected type name after 'Type'")?;
        self.expect_newline("Expected newline after Type declaration")?;

        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndType]) {
            let field_start = self.peek().span;
            let field_name = self.expect_identifier("Expected field name")?;
            self.expect_simple(TokenKind::As, "Expected 'As' in field declaration")?;
            let ty = self.parse_type_name()?;
            let field_end = self.previous().span;
            fields.push(FieldDecl {
                name: field_name,
                ty,
                span: Span::new(field_start.start, field_end.end),
            });
            self.expect_statement_end("Expected newline after field declaration")?;
            self.skip_newlines();
        }

        self.expect_simple(TokenKind::End, "Expected 'End Type'")?;
        let end = self
            .expect_simple(TokenKind::Type, "Expected 'Type' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(TypeDecl {
            name,
            fields,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_procedure(&mut self) -> Result<Procedure, Diagnostic> {
        let start = self.expect_simple(TokenKind::Sub, "Expected 'Sub'")?.span;
        let name = self.expect_identifier("Expected procedure name after 'Sub'")?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after procedure name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after procedure parameters",
        )?;
        self.expect_newline("Expected newline after procedure declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndSub])?;
        self.expect_simple(TokenKind::End, "Expected 'End Sub'")?;
        let end = self
            .expect_simple(TokenKind::Sub, "Expected 'Sub' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Procedure {
            name,
            params,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_function(&mut self) -> Result<Function, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Function, "Expected 'Function'")?
            .span;
        let name = self.expect_identifier("Expected function name after 'Function'")?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after function name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after function parameters",
        )?;
        self.expect_simple(TokenKind::As, "Expected 'As' before function return type")?;
        let return_type = self.parse_type_name()?;
        self.expect_newline("Expected newline after function declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndFunction])?;
        self.expect_simple(TokenKind::End, "Expected 'End Function'")?;
        let end = self
            .expect_simple(TokenKind::Function, "Expected 'Function' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Function {
            name,
            params,
            return_type,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_parameters(&mut self) -> Result<Vec<Parameter>, Diagnostic> {
        let mut params = Vec::new();
        if self.check_simple(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            let (mode, start) = if self.match_simple(&TokenKind::ByVal) {
                (PassingMode::ByVal, self.previous().span)
            } else if self.match_simple(&TokenKind::ByRef) {
                (PassingMode::ByRef, self.previous().span)
            } else {
                return Err(self.error_here("Expected 'ByVal' or 'ByRef' before parameter"));
            };
            let name = self.expect_identifier("Expected parameter name")?;
            self.expect_simple(TokenKind::As, "Expected 'As' in parameter declaration")?;
            let ty = self.parse_type_name()?;
            let end = self.previous().span;
            params.push(Parameter {
                name,
                ty,
                mode,
                span: Span::new(start.start, end.end),
            });

            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    pub(super) fn parse_type_name(&mut self) -> Result<TypeName, Diagnostic> {
        let token = self.advance();
        match token.kind {
            TokenKind::StringType => Ok(TypeName::String),
            TokenKind::IntegerType => Ok(TypeName::Integer),
            TokenKind::BooleanType => Ok(TypeName::Boolean),
            TokenKind::VariantType => Ok(TypeName::Variant),
            TokenKind::Identifier(name) => Ok(TypeName::User(name)),
            _ => Err(Diagnostic::new("Expected type name", Some(token.span))),
        }
    }
}
