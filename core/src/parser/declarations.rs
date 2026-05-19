use super::*;
use crate::runtime::{Diagnostic, Span, TypeName};

impl Parser {
    pub(super) fn parse_import_decl(&mut self) -> Result<ImportDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Import, "Expected 'Import'")?
            .span;
        let module = self.expect_identifier("Expected module name after 'Import'")?;
        let alias = if self.match_simple(&TokenKind::As) {
            Some(self.expect_identifier("Expected import alias after 'As'")?)
        } else {
            None
        };
        let end = self.previous().span;
        self.expect_statement_end("Expected newline after Import declaration")?;
        Ok(ImportDecl {
            module,
            alias,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_class_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<ClassDecl, Diagnostic> {
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
            visibility,
            name,
            members,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        let visibility = self.parse_optional_visibility();
        let with_events = self.match_simple(&TokenKind::WithEvents);
        let is_default = self.match_simple(&TokenKind::Default);
        match self.peek_kind() {
            TokenKind::Event => {
                if is_default {
                    return Err(self.error_here("Default is only supported on Property"));
                }
                if with_events {
                    return Err(self.error_here("WithEvents is only supported on fields"));
                }
                self.parse_event(visibility).map(ClassMember::Event)
            }
            TokenKind::Sub => Ok(ClassMember::Sub(ClassSub {
                visibility,
                procedure: self.parse_procedure(visibility)?,
            })),
            TokenKind::Function => Ok(ClassMember::Function(ClassFunction {
                visibility,
                function: self.parse_function(visibility)?,
            })),
            TokenKind::Property => Ok(ClassMember::Property(
                self.parse_property(visibility, is_default)?,
            )),
            _ if is_default => Err(self.error_here("Default is only supported on Property")),
            TokenKind::Identifier(_) => {
                let start = self.peek().span;
                let name = self.expect_identifier("Expected class field name")?;
                self.expect_simple(TokenKind::As, "Expected 'As' in class field declaration")?;
                let ty = self.parse_type_name()?;
                let end = self.previous().span;
                self.expect_statement_end("Expected newline after class field")?;
                Ok(ClassMember::Field(ClassField {
                    visibility,
                    with_events,
                    name,
                    ty,
                    span: Span::new(start.start, end.end),
                }))
            }
            _ => Err(self.error_here("Expected class member")),
        }
    }

    fn parse_event(&mut self, visibility: Visibility) -> Result<ClassEvent, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Event, "Expected 'Event'")?
            .span;
        let name = self.expect_identifier("Expected event name")?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after event name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(TokenKind::RightParen, "Expected ')' after event parameters")?;
        let end = self.previous().span;
        self.expect_statement_end("Expected newline after event declaration")?;
        Ok(ClassEvent {
            visibility,
            name,
            params,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_property(
        &mut self,
        visibility: Visibility,
        is_default: bool,
    ) -> Result<ClassProperty, Diagnostic> {
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
            is_default,
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

    pub(super) fn parse_module_const(
        &mut self,
        visibility: Visibility,
    ) -> Result<ConstDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Const, "Expected 'Const'")?
            .span;
        let name = self.expect_identifier("Expected constant name")?;
        let ty = if self.match_simple(&TokenKind::As) {
            Some(self.parse_type_name()?)
        } else {
            None
        };
        self.expect_simple(TokenKind::Equal, "Expected '=' in Const declaration")?;
        let value = self.parse_expression()?;
        let end = value.span;
        self.expect_statement_end("Expected newline after Const declaration")?;

        Ok(ConstDecl {
            visibility,
            name,
            ty,
            value,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_module_var(
        &mut self,
        visibility: Visibility,
    ) -> Result<ModuleVarDecl, Diagnostic> {
        let start = self.peek().span;
        let name = self.expect_identifier("Expected module variable name")?;
        let array = if self.match_simple(&TokenKind::LeftParen) {
            if self.match_simple(&TokenKind::RightParen) {
                Some(ArrayDecl::Dynamic)
            } else {
                let size_token = self.advance();
                let TokenKind::Integer(size) = size_token.kind else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "Array size must be an Integer literal",
                        Some(size_token.span),
                    ));
                };
                if size < 0 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "Array size must be non-negative",
                        Some(size_token.span),
                    ));
                }
                self.expect_simple(TokenKind::RightParen, "Expected ')' after array size")?;
                Some(ArrayDecl::Fixed(size))
            }
        } else {
            None
        };
        self.expect_simple(
            TokenKind::As,
            "Expected 'As' in module variable declaration",
        )?;
        let ty = self.parse_type_name()?;
        let end = self.previous().span;
        self.expect_statement_end("Expected newline after module variable declaration")?;

        Ok(ModuleVarDecl {
            visibility,
            name,
            ty,
            array,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_type_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<TypeDecl, Diagnostic> {
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
            visibility,
            name,
            fields,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_procedure(
        &mut self,
        visibility: Visibility,
    ) -> Result<Procedure, Diagnostic> {
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
            visibility,
            name,
            params,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    pub(super) fn parse_function(
        &mut self,
        visibility: Visibility,
    ) -> Result<Function, Diagnostic> {
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
            visibility,
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
            let is_param_array = self.match_simple(&TokenKind::ParamArray);
            let is_optional = if is_param_array {
                false
            } else {
                self.match_simple(&TokenKind::Optional)
            };
            let prefix_start = if is_param_array || is_optional {
                Some(self.previous().span)
            } else {
                None
            };
            let (mode, start) = if self.match_simple(&TokenKind::ByVal) {
                (PassingMode::ByVal, self.previous().span)
            } else if self.match_simple(&TokenKind::ByRef) {
                (PassingMode::ByRef, self.previous().span)
            } else {
                (
                    PassingMode::ByRef,
                    prefix_start.unwrap_or_else(|| self.peek().span),
                )
            };
            let name = self.expect_identifier("Expected parameter name")?;
            let mut array = false;
            if self.match_simple(&TokenKind::LeftParen) {
                self.expect_simple(TokenKind::RightParen, "Expected ')' after parameter array")?;
                array = true;
            }
            self.expect_simple(TokenKind::As, "Expected 'As' in parameter declaration")?;
            let ty = self.parse_type_name()?;
            let optional_default = if is_optional && self.match_simple(&TokenKind::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            if is_param_array && (!array || ty != TypeName::Variant) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "ParamArray must be declared as Variant()",
                    Some(Span::new(start.start, self.previous().span.end)),
                ));
            }
            let end = self.previous().span;
            params.push(Parameter {
                name,
                ty,
                mode,
                is_optional,
                optional_default,
                is_param_array,
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
            TokenKind::Identifier(mut name) => {
                if self.match_simple(&TokenKind::Dot) {
                    let member =
                        self.expect_identifier("Expected type name after module qualifier")?;
                    name.push('.');
                    name.push_str(&member);
                }
                Ok(TypeName::User(name))
            }
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Expected type name",
                Some(token.span),
            )),
        }
    }
}
