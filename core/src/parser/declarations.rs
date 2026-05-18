use super::*;
use crate::runtime::{Diagnostic, Span, TypeName};

impl Parser {
    pub(super) fn parse_class_decl(&mut self) -> Result<ClassDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Class, "Expected 'Class'")?
            .span;
        let name = self.expect_identifier("Expected class name after 'Class'")?;
        self.expect_newline("Expected newline after Class declaration")?;

        let mut members = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndClass]) {
            members.push(self.parse_class_member()?);
            self.skip_newlines();
        }

        self.expect_simple(TokenKind::End, "Expected 'End Class'")?;
        let end = self
            .expect_simple(TokenKind::Class, "Expected 'Class' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(ClassDecl {
            name,
            members,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        let visibility = self.parse_optional_visibility();
        match self.peek_kind() {
            TokenKind::Sub => Ok(ClassMember::Sub(ClassSub {
                visibility,
                procedure: self.parse_procedure()?,
            })),
            TokenKind::Function => Ok(ClassMember::Function(ClassFunction {
                visibility,
                function: self.parse_function()?,
            })),
            TokenKind::Property => Ok(ClassMember::Property(self.parse_property(visibility)?)),
            TokenKind::Identifier(_) => {
                let start = self.peek().span;
                let name = self.expect_identifier("Expected class field name")?;
                self.expect_simple(TokenKind::As, "Expected 'As' in class field declaration")?;
                let ty = self.parse_type_name()?;
                let end = self.previous().span;
                self.expect_statement_end("Expected newline after class field")?;
                Ok(ClassMember::Field(ClassField {
                    visibility,
                    name,
                    ty,
                    span: Span::new(start.start, end.end),
                }))
            }
            _ => Err(self.error_here("Expected class member")),
        }
    }

    fn parse_property(&mut self, visibility: Visibility) -> Result<ClassProperty, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Property, "Expected 'Property'")?
            .span;
        let kind = if self.match_simple(&TokenKind::Get) {
            PropertyKind::Get
        } else if self.match_simple(&TokenKind::Let) {
            PropertyKind::Let
        } else if self.match_simple(&TokenKind::Set) {
            PropertyKind::Set
        } else {
            return Err(self.error_here("Expected 'Get', 'Let', or 'Set' after 'Property'"));
        };
        let name = self.expect_identifier("Expected property name")?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after property name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after property parameters",
        )?;
        let return_type = if kind == PropertyKind::Get {
            self.expect_simple(TokenKind::As, "Expected 'As' before property return type")?;
            Some(self.parse_type_name()?)
        } else {
            None
        };
        self.expect_newline("Expected newline after property declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndProperty])?;
        self.expect_simple(TokenKind::End, "Expected 'End Property'")?;
        let end = self
            .expect_simple(TokenKind::Property, "Expected 'Property' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(ClassProperty {
            visibility,
            name,
            kind,
            params,
            return_type,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_optional_visibility(&mut self) -> Visibility {
        if self.match_simple(&TokenKind::Private) {
            Visibility::Private
        } else {
            self.match_simple(&TokenKind::Public);
            Visibility::Public
        }
    }

    pub(super) fn parse_enum_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<EnumDecl, Diagnostic> {
        let start = self.expect_simple(TokenKind::Enum, "Expected 'Enum'")?.span;
        let name = self.expect_identifier("Expected enum name after 'Enum'")?;
        self.expect_newline("Expected newline after Enum declaration")?;

        let mut members = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndEnum]) {
            let member_start = self.peek().span;
            let member_name = self.expect_identifier("Expected enum member name")?;
            let value = if self.match_simple(&TokenKind::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let end = value.as_ref().map(|expr| expr.span).unwrap_or(member_start);
            members.push(EnumMemberDecl {
                name: member_name,
                value,
                span: Span::new(member_start.start, end.end),
            });
            self.expect_statement_end("Expected newline after enum member")?;
            self.skip_newlines();
        }

        self.expect_simple(TokenKind::End, "Expected 'End Enum'")?;
        let end = self
            .expect_simple(TokenKind::Enum, "Expected 'Enum' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(EnumDecl {
            visibility,
            name,
            members,
            span: Span::new(start.start, end.end),
        })
    }

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
