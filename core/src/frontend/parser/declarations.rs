use super::*;
use crate::runtime::{Diagnostic, Span, TypeName};

impl Parser {
    pub(super) fn parse_attribute_decl(&mut self) -> Result<AttributeDecl, Diagnostic> {
        let start = self.expect_identifier("Expected 'Attribute'")?;
        debug_assert!(start.eq_ignore_ascii_case("Attribute"));
        let start_span = self.previous().span;
        let mut target = self.expect_identifier("Expected Attribute target")?;
        while self.match_simple(&TokenKind::Dot) {
            target.push('.');
            target.push_str(&self.expect_identifier("Expected Attribute name after '.'")?);
        }
        self.expect_simple(TokenKind::Equal, "Expected '=' in Attribute declaration")?;
        let value_token = self.advance();
        let value = match value_token.kind {
            TokenKind::Integer(value) => value.to_string(),
            TokenKind::Minus => {
                let integer = self.expect_simple(
                    TokenKind::Integer(0),
                    "Expected integer literal after '-' in Attribute value",
                )?;
                let TokenKind::Integer(value) = integer.kind else {
                    unreachable!();
                };
                format!("-{value}")
            }
            TokenKind::String(value) | TokenKind::Identifier(value, _) => value,
            TokenKind::True => "True".to_string(),
            TokenKind::False => "False".to_string(),
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected Attribute value",
                    Some(value_token.span),
                ));
            }
        };
        self.expect_statement_end("Expected newline after Attribute declaration")?;
        let (target, name) = target
            .rsplit_once('.')
            .map(|(target, name)| (target.to_string(), name.to_string()))
            .unwrap_or_else(|| (String::new(), target));
        Ok(AttributeDecl {
            target,
            name,
            value,
            span: Span::new(self.file_id, start_span.start, value_token.span.end),
        })
    }

    pub(super) fn parse_import_decl(&mut self) -> Result<ImportDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Import, "Expected 'Import'")?
            .span;
        let token = self.advance();
        let module = match token.kind {
            TokenKind::String(value) => value,
            TokenKind::Identifier(name, _) => {
                let mut path = name;
                while self.match_simple(&TokenKind::Dot) {
                    let next =
                        self.expect_identifier("Expected identifier after '.' in import path")?;
                    path.push('.');
                    path.push_str(&next);
                }
                path
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected module name or string literal after 'Import'",
                    Some(token.span),
                ));
            }
        };

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
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    pub(super) fn parse_class_decl(
        &mut self,
        visibility: Visibility,
        inheritance: ClassInheritance,
    ) -> Result<ClassDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Class, "Expected 'Class'")?
            .span;
        let name = self.expect_identifier("Expected class name after 'Class'")?;
        let type_params = self.parse_optional_type_params()?;
        let mut base_class = None;
        if self.match_simple(&TokenKind::Inherits) {
            base_class = Some(self.parse_type_name()?);
        }
        let mut implements = Vec::new();
        if self.match_simple(&TokenKind::Implements) {
            loop {
                implements.push(self.parse_type_name()?);
                if !self.match_simple(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect_statement_end("Expected newline after Class declaration")?;

        let mut members = Vec::new();
        let mut attributes = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndClass]) {
            if matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute"))
            {
                let attribute = self.parse_attribute_decl()?;
                self.apply_class_attribute(&attribute, &mut members);
                attributes.push(attribute);
            } else {
                members.push(self.parse_class_member()?);
            }
            self.skip_newlines();
        }

        self.expect_simple(TokenKind::End, "Expected 'End Class'")?;
        let end = self
            .expect_simple(TokenKind::Class, "Expected 'Class' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(ClassDecl {
            visibility,
            inheritance,
            name,
            type_params,
            base_class,
            implements,
            attributes,
            members,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    pub(super) fn parse_interface_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<InterfaceDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Interface, "Expected 'Interface'")?
            .span;
        let name = self.expect_identifier("Expected interface name after 'Interface'")?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_statement_end("Expected newline after Interface declaration")?;
        let mut members = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndInterface]) {
            members.push(self.parse_interface_member()?);
            self.skip_newlines();
        }
        self.expect_simple(TokenKind::End, "Expected 'End Interface'")?;
        let end = self
            .expect_simple(TokenKind::Interface, "Expected 'Interface' after 'End'")?
            .span;
        self.consume_statement_end();
        Ok(InterfaceDecl {
            visibility,
            name,
            type_params,
            members,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_interface_member(&mut self) -> Result<InterfaceMember, Diagnostic> {
        let _visibility = self
            .parse_optional_visibility()
            .unwrap_or(Visibility::Public);
        match self.peek_kind() {
            TokenKind::Sub => {
                let start = self.expect_simple(TokenKind::Sub, "Expected 'Sub'")?.span;
                let name = self.expect_identifier("Expected interface Sub name")?;
                self.expect_simple(TokenKind::LeftParen, "Expected '(' after Sub name")?;
                let params = self.parse_parameters()?;
                self.expect_simple(TokenKind::RightParen, "Expected ')' after parameters")?;
                let end = self.previous().span;
                self.expect_statement_end("Expected newline after interface Sub")?;
                Ok(InterfaceMember::Sub(InterfaceMethod {
                    name,
                    params,
                    return_type: None,
                    span: Span::new(self.file_id, start.start, end.end),
                }))
            }
            TokenKind::Function => {
                let start = self
                    .expect_simple(TokenKind::Function, "Expected 'Function'")?
                    .span;
                let name = self.expect_identifier("Expected interface Function name")?;
                self.expect_simple(TokenKind::LeftParen, "Expected '(' after Function name")?;
                let params = self.parse_parameters()?;
                self.expect_simple(TokenKind::RightParen, "Expected ')' after parameters")?;
                self.expect_simple(TokenKind::As, "Expected 'As' before return type")?;
                let return_type = self.parse_type_name()?;
                let end = self.previous().span;
                self.expect_statement_end("Expected newline after interface Function")?;
                Ok(InterfaceMember::Function(InterfaceMethod {
                    name,
                    params,
                    return_type: Some(return_type),
                    span: Span::new(self.file_id, start.start, end.end),
                }))
            }
            TokenKind::Property => {
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
                let name = self.expect_identifier("Expected interface Property name")?;
                self.expect_simple(TokenKind::LeftParen, "Expected '(' after property name")?;
                let params = self.parse_parameters()?;
                self.expect_simple(TokenKind::RightParen, "Expected ')' after parameters")?;
                let return_type = if kind == PropertyKind::Get {
                    self.expect_simple(TokenKind::As, "Expected 'As' before property type")?;
                    Some(self.parse_type_name()?)
                } else {
                    None
                };
                let end = self.previous().span;
                self.expect_statement_end("Expected newline after interface Property")?;
                Ok(InterfaceMember::Property(InterfaceProperty {
                    name,
                    kind,
                    params,
                    return_type,
                    span: Span::new(self.file_id, start.start, end.end),
                }))
            }
            TokenKind::Event => {
                let event = self.parse_event(Visibility::Public)?;
                Ok(InterfaceMember::Event(InterfaceEvent {
                    name: event.name,
                    params: event.params,
                    span: event.span,
                }))
            }
            _ => Err(self.error_here("Expected interface member")),
        }
    }

    pub(super) fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        let explicit_visibility = self.parse_optional_visibility();
        let visibility = explicit_visibility.unwrap_or(Visibility::Public);
        let override_kind = self.parse_optional_override_kind();
        let is_shared = self.match_simple(&TokenKind::Shared);
        let with_events = self.match_simple(&TokenKind::WithEvents);
        let is_default = self.match_simple(&TokenKind::Default);
        let is_iterator = self.match_simple(&TokenKind::Iterator);
        match self.peek_kind() {
            TokenKind::Event => {
                if is_default {
                    return Err(self.error_here("Default is only supported on Property"));
                }
                if is_iterator {
                    return Err(self.error_here("Iterator is not supported on Event"));
                }
                if with_events {
                    return Err(self.error_here("WithEvents is only supported on fields"));
                }
                self.parse_event(visibility).map(ClassMember::Event)
            }
            TokenKind::Const => self.parse_module_consts(visibility).and_then(|mut consts| {
                if consts.len() == 1 {
                    Ok(ClassMember::Const(consts.remove(0)))
                } else {
                    Err(self.error_here("Class Const declarations must be one per member"))
                }
            }),
            TokenKind::Sub => {
                if is_iterator {
                    return Err(self.error_here("Iterator is not supported on Sub"));
                }
                if matches!(self.peek_next_kind(), Some(TokenKind::New)) {
                    Ok(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind,
                        is_shared,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "New",
                            "Initialize",
                        )?,
                    }))
                } else if matches!(
                    self.peek_next_kind(),
                    Some(TokenKind::Identifier(name, _)) if name.eq_ignore_ascii_case("Terminate")
                ) {
                    Ok(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind,
                        is_shared,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "Terminate",
                            "Terminate",
                        )?,
                    }))
                } else {
                    let (procedure, implements) = self.parse_class_procedure(visibility)?;
                    Ok(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind,
                        is_shared,
                        implements,
                        procedure,
                    }))
                }
            }
            TokenKind::Function => {
                let mut function = self.parse_class_function(visibility, is_iterator)?;
                function.override_kind = override_kind;
                function.is_shared = is_shared;
                Ok(ClassMember::Function(function))
            }
            TokenKind::Property => {
                let mut property = self.parse_property(visibility, is_default, is_iterator)?;
                property.override_kind = override_kind;
                property.is_shared = is_shared;
                Ok(ClassMember::Property(property))
            }
            TokenKind::Type | TokenKind::Structure => {
                self.parse_type_decl(visibility).map(ClassMember::Type)
            }
            TokenKind::Enum => self.parse_enum_decl(visibility).map(ClassMember::Enum),
            TokenKind::Declare => self
                .parse_declare_decl(visibility)
                .map(ClassMember::Declare),
            _ if is_iterator => {
                Err(self.error_here("Expected Function or Property after Iterator"))
            }
            _ if is_default => Err(self.error_here("Default is only supported on Property")),
            TokenKind::Identifier(_, _) | TokenKind::Dim => {
                if is_iterator {
                    return Err(self.error_here("Iterator is not supported on fields"));
                }
                let has_dim = self.match_simple(&TokenKind::Dim);
                if has_dim && with_events {
                    return Err(self.error_here("WithEvents is not supported with Dim fields"));
                }
                let visibility = explicit_visibility.unwrap_or(if has_dim {
                    Visibility::Private
                } else {
                    Visibility::Public
                });
                let decls = self.parse_variable_declarators("field")?;
                self.expect_statement_end("Expected newline after class field")?;
                let fields = decls
                    .into_iter()
                    .map(|decl| ClassField {
                        visibility,
                        is_shared,
                        with_events,
                        name: decl.name,
                        ty: decl.ty,
                        array: decl.array,
                        initializer: decl.initializer,
                        span: decl.span,
                    })
                    .collect::<Vec<_>>();
                if fields.len() == 1 {
                    Ok(ClassMember::Field(
                        fields.into_iter().next().expect("len checked"),
                    ))
                } else {
                    Ok(ClassMember::Fields(fields))
                }
            }
            _ => Err(self.error_here("Expected class member")),
        }
    }

    fn parse_optional_implements_clause(&mut self) -> Result<Vec<ImplementsClause>, Diagnostic> {
        let mut clauses = Vec::new();
        if !self.match_simple(&TokenKind::Implements) {
            return Ok(clauses);
        }
        loop {
            let start = self.previous().span;
            let interface_name = TypeName::User(self.expect_identifier("Expected interface name")?);
            self.expect_simple(TokenKind::Dot, "Expected '.' in Implements clause")?;
            let member_name = self.expect_identifier("Expected interface member name")?;
            let end = self.previous().span;
            clauses.push(ImplementsClause {
                interface_name,
                member_name,
                span: Span::new(self.file_id, start.start, end.end),
            });
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        Ok(clauses)
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
            is_shared: false,
            name,
            params,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    pub(super) fn parse_property(
        &mut self,
        visibility: Visibility,
        is_default: bool,
        is_iterator: bool,
    ) -> Result<ClassProperty, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Property, "Expected 'Property'")?
            .span;
        let mut is_default = is_default;
        let mut is_enumerator = false;
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
            if self.match_simple(&TokenKind::As) {
                Some(self.parse_type_name()?)
            } else {
                Some(crate::runtime::TypeName::Variant)
            }
        } else {
            None
        };
        let implements = self.parse_optional_implements_clause()?;
        self.expect_statement_end("Expected newline after property declaration")?;
        while matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute"))
        {
            let attribute = self.parse_attribute_decl()?;
            if attribute.target.eq_ignore_ascii_case(&name)
                && attribute.name.eq_ignore_ascii_case("VB_UserMemId")
            {
                if attribute.value == "0" {
                    is_default = true;
                } else if attribute.value == "-4" {
                    is_enumerator = true;
                }
            }
            self.skip_newlines();
        }

        let body = self.parse_block_until(&[BlockEnd::EndProperty])?;
        self.expect_simple(TokenKind::End, "Expected 'End Property'")?;
        let end = self
            .expect_simple(TokenKind::Property, "Expected 'Property' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(ClassProperty {
            visibility,
            override_kind: OverrideKind::None,
            is_shared: false,
            implements,
            is_default,
            is_enumerator,
            is_iterator,
            name,
            kind,
            params,
            return_type,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    pub(super) fn parse_optional_visibility(&mut self) -> Option<Visibility> {
        if self.match_simple(&TokenKind::Private) {
            Some(Visibility::Private)
        } else if self.match_simple(&TokenKind::Friend) {
            Some(Visibility::Friend)
        } else if self.match_simple(&TokenKind::Public) {
            Some(Visibility::Public)
        } else if self.match_simple(&TokenKind::Protected) {
            if self.match_simple(&TokenKind::Friend) {
                Some(Visibility::ProtectedFriend)
            } else {
                Some(Visibility::Protected)
            }
        } else {
            None
        }
    }

    pub(super) fn parse_optional_class_inheritance(&mut self) -> ClassInheritance {
        if self.match_simple(&TokenKind::MustInherit) {
            ClassInheritance::MustInherit
        } else if self.match_simple(&TokenKind::NotInheritable) {
            ClassInheritance::NotInheritable
        } else {
            ClassInheritance::Normal
        }
    }

    fn parse_optional_override_kind(&mut self) -> OverrideKind {
        if self.match_simple(&TokenKind::Overridable) {
            OverrideKind::Overridable
        } else if self.match_simple(&TokenKind::Overrides) {
            OverrideKind::Overrides
        } else if self.match_simple(&TokenKind::MustOverride) {
            OverrideKind::MustOverride
        } else if self.match_simple(&TokenKind::Shadows) {
            OverrideKind::Shadows
        } else {
            OverrideKind::None
        }
    }

    pub(super) fn parse_enum_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<EnumDecl, Diagnostic> {
        let start = self.expect_simple(TokenKind::Enum, "Expected 'Enum'")?.span;
        let name = self.expect_identifier("Expected enum name after 'Enum'")?;
        self.expect_statement_end("Expected newline after Enum declaration")?;

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
                span: Span::new(self.file_id, member_start.start, end.end),
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
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    pub(super) fn parse_module_consts(
        &mut self,
        visibility: Visibility,
    ) -> Result<Vec<ConstDecl>, Diagnostic> {
        let consts = self.parse_const_declarators(visibility)?;
        self.expect_statement_end("Expected newline after Const declaration")?;
        Ok(consts)
    }

    pub(super) fn parse_const_declarators(
        &mut self,
        visibility: Visibility,
    ) -> Result<Vec<ConstDecl>, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Const, "Expected 'Const'")?
            .span;
        let mut consts = Vec::new();
        loop {
            let item_start = self.peek().span;
            let token = self.advance();
            let (name, hint) = match token.kind {
                TokenKind::Identifier(name, hint) => (name, hint),
                TokenKind::Version => ("VERSION".to_string(), None),
                _ => return Err(self.error_here("Expected constant name")),
            };
            let ty = if self.match_simple(&TokenKind::As) {
                let as_ty = self.parse_type_name()?;
                if let Some(ref h) = hint
                    && !h.same_type(&as_ty)
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Type-declaration character implies {}, but As clause declares {}",
                            h.display_name(),
                            as_ty.display_name()
                        ),
                        Some(item_start),
                    ));
                }
                Some(as_ty)
            } else {
                hint
            };
            self.expect_simple(TokenKind::Equal, "Expected '=' in Const declaration")?;
            let value = self.parse_expression()?;
            let end = value.span;
            consts.push(ConstDecl {
                visibility,
                name,
                ty,
                value,
                span: Span::new(self.file_id, item_start.start, end.end),
            });
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        if let Some(first) = consts.first_mut() {
            first.span.start = start.start;
        }
        Ok(consts)
    }

    pub(super) fn parse_declare_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<DeclareDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Declare, "Expected 'Declare'")?
            .span;
        let ptr_safe = self.match_simple(&TokenKind::PtrSafe);
        let kind = if self.match_simple(&TokenKind::Function) {
            DeclareKind::Function
        } else if self.match_simple(&TokenKind::Sub) {
            DeclareKind::Sub
        } else {
            return Err(self.error_here("Expected Function or Sub after Declare"));
        };
        let name = self.expect_identifier("Expected Declare name")?;
        self.expect_simple(TokenKind::Lib, "Expected Lib in Declare")?;
        let lib = match self.advance().kind {
            TokenKind::String(value) => value,
            _ => return Err(self.error_here("Expected library string after Lib")),
        };
        let alias = if self.match_simple(&TokenKind::Alias) {
            match self.advance().kind {
                TokenKind::String(value) => Some(value),
                _ => return Err(self.error_here("Expected alias string after Alias")),
            }
        } else {
            None
        };
        let calling_convention = if self.match_identifier("CDecl") {
            CallingConvention::CDecl
        } else if self.match_identifier("StdCall") {
            CallingConvention::StdCall
        } else {
            CallingConvention::Default
        };
        self.expect_simple(TokenKind::LeftParen, "Expected '(' in Declare")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after Declare parameters",
        )?;
        let return_type = if kind == DeclareKind::Function {
            self.expect_simple(TokenKind::As, "Expected 'As' before Declare return type")?;
            Some(self.parse_type_name()?)
        } else {
            None
        };
        let end = self.previous().span;
        self.expect_statement_end("Expected newline after Declare")?;
        Ok(DeclareDecl {
            visibility,
            ptr_safe,
            calling_convention,
            kind,
            name,
            lib,
            alias,
            params,
            return_type,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    pub(super) fn parse_module_vars(
        &mut self,
        visibility: Visibility,
    ) -> Result<Vec<ModuleVarDecl>, Diagnostic> {
        let decls = self.parse_variable_declarators("module variable")?;
        self.expect_statement_end("Expected newline after module variable declaration")?;

        Ok(decls
            .into_iter()
            .map(|decl| ModuleVarDecl {
                visibility,
                name: decl.name,
                ty: decl.ty,
                array: decl.array,
                initializer: decl.initializer,
                span: decl.span,
            })
            .collect())
    }

    pub(super) fn parse_type_decl(
        &mut self,
        visibility: Visibility,
    ) -> Result<TypeDecl, Diagnostic> {
        let (start, kind, keyword, end_block) = if self.match_simple(&TokenKind::Type) {
            (
                self.previous().span,
                TypeKind::Type,
                "Type",
                BlockEnd::EndType,
            )
        } else {
            (
                self.expect_simple(TokenKind::Structure, "Expected 'Type' or 'Structure'")?
                    .span,
                TypeKind::Structure,
                "Structure",
                BlockEnd::EndStructure,
            )
        };
        let name = self.expect_identifier(&format!("Expected type name after '{keyword}'"))?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_statement_end(&format!("Expected newline after {keyword} declaration"))?;

        let mut fields = Vec::new();
        let mut members = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[end_block]) {
            if kind == TypeKind::Type && self.type_member_starts_non_field() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    "Type declarations support fields only; use Structure for methods and properties",
                    Some(self.peek().span),
                ));
            }

            if kind == TypeKind::Structure && self.structure_member_starts_non_field() {
                members.push(self.parse_structure_member()?);
            } else {
                let explicit_visibility = self.parse_optional_visibility();
                if self.match_simple(&TokenKind::WithEvents) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        "Structure fields cannot use WithEvents",
                        Some(self.previous().span),
                    ));
                }
                let has_dim = self.match_simple(&TokenKind::Dim);
                let visibility = explicit_visibility.unwrap_or(if has_dim {
                    Visibility::Private
                } else {
                    Visibility::Public
                });
                let decls = self.parse_variable_declarators("field")?;
                self.expect_statement_end("Expected newline after field declaration")?;
                for decl in decls {
                    let Some(ty) = decl.ty else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "Structure and Type fields must declare an explicit type",
                            Some(decl.span),
                        ));
                    };
                    if kind == TypeKind::Type && decl.initializer.is_some() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "VBA Type field initializers are not supported",
                            Some(decl.span),
                        ));
                    }
                    fields.push(FieldDecl {
                        visibility,
                        name: decl.name,
                        ty,
                        array: decl.array,
                        initializer: decl.initializer,
                        span: decl.span,
                    });
                }
            }
            self.skip_newlines();
        }

        self.expect_simple(TokenKind::End, &format!("Expected 'End {keyword}'"))?;
        let end = if keyword == "Type" {
            self.expect_simple(TokenKind::Type, "Expected 'Type' after 'End'")?
                .span
        } else {
            self.expect_simple(TokenKind::Structure, "Expected 'Structure' after 'End'")?
                .span
        };
        self.consume_statement_end();

        Ok(TypeDecl {
            visibility,
            kind,
            name,
            type_params,
            fields,
            members,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn type_member_starts_non_field(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Sub
            | TokenKind::Function
            | TokenKind::Iterator
            | TokenKind::Property
            | TokenKind::Default
            | TokenKind::Event
            | TokenKind::WithEvents => true,
            TokenKind::Public | TokenKind::Private => matches!(
                self.peek_next_kind(),
                Some(
                    TokenKind::Sub
                        | TokenKind::Function
                        | TokenKind::Iterator
                        | TokenKind::Property
                        | TokenKind::Default
                        | TokenKind::Event
                        | TokenKind::WithEvents
                )
            ),
            _ => false,
        }
    }

    fn structure_member_starts_non_field(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Sub
            | TokenKind::Function
            | TokenKind::Iterator
            | TokenKind::Property
            | TokenKind::Default
            | TokenKind::Event => true,
            TokenKind::Public | TokenKind::Private => matches!(
                self.peek_next_kind(),
                Some(
                    TokenKind::Sub
                        | TokenKind::Function
                        | TokenKind::Iterator
                        | TokenKind::Property
                        | TokenKind::Default
                        | TokenKind::Event
                )
            ),
            _ => false,
        }
    }

    fn parse_structure_member(&mut self) -> Result<ClassMember, Diagnostic> {
        let visibility = self
            .parse_optional_visibility()
            .unwrap_or(Visibility::Public);
        let is_default = self.match_simple(&TokenKind::Default);
        match self.peek_kind() {
            TokenKind::Event => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Structure cannot declare events",
                Some(self.peek().span),
            )),
            TokenKind::Iterator => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Structure cannot declare Iterator members",
                Some(self.peek().span),
            )),
            TokenKind::Sub => {
                if matches!(self.peek_next_kind(), Some(TokenKind::New)) {
                    Ok(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared: false,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "New",
                            "Initialize",
                        )?,
                    }))
                } else if matches!(
                    self.peek_next_kind(),
                    Some(TokenKind::Identifier(name, _)) if name.eq_ignore_ascii_case("Constructor")
                ) {
                    Ok(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared: false,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "Constructor",
                            "Initialize",
                        )?,
                    }))
                } else {
                    Ok(ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared: false,
                        implements: Vec::new(),
                        procedure: self.parse_procedure(visibility)?,
                    }))
                }
            }
            TokenKind::Function => Ok(ClassMember::Function(
                self.parse_class_function(visibility, false)?,
            )),
            TokenKind::Property => Ok(ClassMember::Property(
                self.parse_property(visibility, is_default, false)?,
            )),
            _ if is_default => Err(self.error_here("Default is only supported on Property")),
            _ => Err(self.error_here("Expected structure member")),
        }
    }

    pub(super) fn parse_procedure(
        &mut self,
        visibility: Visibility,
    ) -> Result<Procedure, Diagnostic> {
        let start = self.expect_simple(TokenKind::Sub, "Expected 'Sub'")?.span;
        if self.match_simple(&TokenKind::New) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Sub New is only allowed inside Class",
                Some(self.previous().span),
            ));
        }
        let name = self.expect_identifier("Expected procedure name after 'Sub'")?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after procedure name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after procedure parameters",
        )?;
        self.expect_statement_end("Expected newline after procedure declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndSub])?;
        self.expect_simple(TokenKind::End, "Expected 'End Sub'")?;
        let end = self
            .expect_simple(TokenKind::Sub, "Expected 'Sub' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Procedure {
            visibility,
            name,
            type_params,
            params,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_class_procedure(
        &mut self,
        visibility: Visibility,
    ) -> Result<(Procedure, Vec<ImplementsClause>), Diagnostic> {
        let start = self.expect_simple(TokenKind::Sub, "Expected 'Sub'")?.span;
        let name = self.expect_identifier("Expected procedure name after 'Sub'")?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after procedure name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after procedure parameters",
        )?;
        let implements = self.parse_optional_implements_clause()?;
        self.expect_statement_end("Expected newline after procedure declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndSub])?;
        self.expect_simple(TokenKind::End, "Expected 'End Sub'")?;
        let end = self
            .expect_simple(TokenKind::Sub, "Expected 'Sub' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok((
            Procedure {
                visibility,
                name,
                type_params,
                params,
                body,
                span: Span::new(self.file_id, start.start, end.end),
            },
            implements,
        ))
    }

    fn parse_lifecycle_sub_procedure(
        &mut self,
        visibility: Visibility,
        syntax_name: &str,
        canonical_name: &str,
    ) -> Result<Procedure, Diagnostic> {
        let start_span = self.expect_simple(TokenKind::Sub, "Expected 'Sub'")?.span;
        let matched_name = if syntax_name.eq_ignore_ascii_case("New") {
            self.match_simple(&TokenKind::New)
        } else if matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case(syntax_name))
        {
            self.advance();
            true
        } else {
            false
        };
        if !matched_name {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                format!("Expected '{syntax_name}' after 'Sub'"),
                Some(self.previous().span),
            ));
        }
        self.expect_simple(
            TokenKind::LeftParen,
            &format!("Expected '(' after {syntax_name}"),
        )?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            &format!("Expected ')' after {syntax_name} parameters"),
        )?;
        self.expect_statement_end(&format!("Expected newline after {syntax_name} declaration"))?;

        let body = self.parse_block_until(&[BlockEnd::EndSub])?;
        self.expect_simple(
            TokenKind::End,
            &format!("Expected 'End Sub' after Sub {syntax_name}"),
        )?;
        let end = self
            .expect_simple(TokenKind::Sub, "Expected 'Sub' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Procedure {
            visibility,
            name: canonical_name.to_string(),
            type_params: Vec::new(),
            params,
            body,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    pub(super) fn parse_function(
        &mut self,
        visibility: Visibility,
        is_iterator: bool,
    ) -> Result<Function, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Function, "Expected 'Function'")?
            .span;
        let name = self.expect_identifier("Expected function name after 'Function'")?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after function name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after function parameters",
        )?;
        let mut return_type = if self.match_simple(&TokenKind::As) {
            self.parse_type_name()?
        } else {
            TypeName::Variant
        };
        if self.match_simple(&TokenKind::LeftParen) {
            self.expect_simple(
                TokenKind::RightParen,
                "Expected ')' after array return type",
            )?;
            return_type = TypeName::Array(Box::new(return_type));
        }
        self.expect_statement_end("Expected newline after function declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndFunction])?;
        self.expect_simple(TokenKind::End, "Expected 'End Function'")?;
        let end = self
            .expect_simple(TokenKind::Function, "Expected 'Function' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Function {
            visibility,
            name,
            is_iterator,
            type_params,
            params,
            return_type,
            return_slot: None,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_class_function(
        &mut self,
        visibility: Visibility,
        is_iterator: bool,
    ) -> Result<ClassFunction, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Function, "Expected 'Function'")?
            .span;
        let name = self.expect_identifier("Expected function name after 'Function'")?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after function name")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after function parameters",
        )?;
        let mut return_type = if self.match_simple(&TokenKind::As) {
            self.parse_type_name()?
        } else {
            TypeName::Variant
        };
        if self.match_simple(&TokenKind::LeftParen) {
            self.expect_simple(
                TokenKind::RightParen,
                "Expected ')' after array return type",
            )?;
            return_type = TypeName::Array(Box::new(return_type));
        }
        let implements = self.parse_optional_implements_clause()?;
        self.expect_statement_end("Expected newline after function declaration")?;

        let mut is_enumerator = false;
        while matches!(self.peek_kind(), TokenKind::Identifier(attribute, _) if attribute.eq_ignore_ascii_case("Attribute"))
        {
            let attribute = self.parse_attribute_decl()?;
            if attribute.target.eq_ignore_ascii_case(&name)
                && attribute.name.eq_ignore_ascii_case("VB_UserMemId")
                && attribute.value == "-4"
            {
                is_enumerator = true;
            }
            self.skip_newlines();
        }

        let body = self.parse_block_until(&[BlockEnd::EndFunction])?;
        self.expect_simple(TokenKind::End, "Expected 'End Function'")?;
        let end = self
            .expect_simple(TokenKind::Function, "Expected 'Function' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(ClassFunction {
            visibility,
            override_kind: OverrideKind::None,
            is_shared: false,
            implements,
            is_enumerator,
            function: Function {
                visibility,
                name,
                is_iterator,
                type_params,
                params,
                return_type,
                return_slot: None,
                body,
                span: Span::new(self.file_id, start.start, end.end),
            },
        })
    }

    pub(super) fn parse_parameters(&mut self) -> Result<Vec<Parameter>, Diagnostic> {
        let mut params = Vec::new();
        self.skip_newlines();
        if self.check_simple(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            self.skip_newlines();
            if self.check_simple(&TokenKind::RightParen) {
                break;
            }
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
            let token = self.advance();
            let (name, hint) = Self::parameter_name_from_token(token.kind)
                .ok_or_else(|| self.error_here("Expected parameter name"))?;
            let mut array = false;
            if self.match_simple(&TokenKind::LeftParen) {
                self.expect_simple(TokenKind::RightParen, "Expected ')' after parameter array")?;
                array = true;
            }
            let ty = if self.match_simple(&TokenKind::As) {
                let as_ty = self.parse_type_name()?;
                if let Some(ref h) = hint
                    && !h.same_type(&as_ty)
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        format!(
                            "Type-declaration character implies {}, but As clause declares {}",
                            h.display_name(),
                            as_ty.display_name()
                        ),
                        Some(token.span),
                    ));
                }
                as_ty
            } else {
                hint.unwrap_or(TypeName::Variant)
            };
            let optional_default = if is_optional && self.match_simple(&TokenKind::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            if is_param_array && (!array || ty != TypeName::Variant) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::ARRAY,
                    "ParamArray must be declared as Variant()",
                    Some(Span::new(
                        self.file_id,
                        start.start,
                        self.previous().span.end,
                    )),
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
                span: Span::new(self.file_id, start.start, end.end),
            });

            self.skip_newlines();
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        Ok(params)
    }

    fn parameter_name_from_token(kind: TokenKind) -> Option<(String, Option<TypeName>)> {
        match kind {
            TokenKind::Identifier(name, hint) => Some((name, hint)),
            TokenKind::Version => Some(("Version".to_string(), None)),
            TokenKind::StringType => Some(("String".to_string(), None)),
            TokenKind::Type => Some(("Type".to_string(), None)),
            TokenKind::Error => Some(("Error".to_string(), None)),
            TokenKind::Text => Some(("Text".to_string(), None)),
            TokenKind::Compare => Some(("Compare".to_string(), None)),
            TokenKind::Binary => Some(("Binary".to_string(), None)),
            TokenKind::Base => Some(("Base".to_string(), None)),
            TokenKind::Lib => Some(("Lib".to_string(), None)),
            _ => None,
        }
    }

    pub(super) fn parse_type_name(&mut self) -> Result<TypeName, Diagnostic> {
        let token = self.advance();
        let ty = match token.kind {
            TokenKind::StringType => {
                if self.match_simple(&TokenKind::Star) {
                    self.expect_simple(TokenKind::Integer(0), "Expected length after '*'")?;
                }
                Ok(TypeName::String)
            }
            TokenKind::IntegerType => Ok(TypeName::Integer),
            TokenKind::BooleanType => Ok(TypeName::Boolean),
            TokenKind::VariantType => Ok(TypeName::Variant),
            TokenKind::Identifier(mut name, _) => {
                if name.eq_ignore_ascii_case("Byte") {
                    Ok(TypeName::Byte)
                } else if name.eq_ignore_ascii_case("Long") {
                    Ok(TypeName::Long)
                } else if name.eq_ignore_ascii_case("Int64") {
                    Ok(TypeName::Int64)
                } else if name.eq_ignore_ascii_case("UInt32") {
                    Ok(TypeName::UInt32)
                } else if name.eq_ignore_ascii_case("UInt64") {
                    Ok(TypeName::UInt64)
                } else if name.eq_ignore_ascii_case("Single") {
                    Ok(TypeName::Single)
                } else if name.eq_ignore_ascii_case("Double") {
                    Ok(TypeName::Double)
                } else if name.eq_ignore_ascii_case("Currency") {
                    Ok(TypeName::Currency)
                } else if name.eq_ignore_ascii_case("Decimal") {
                    Ok(TypeName::Decimal)
                } else if name.eq_ignore_ascii_case("Date") {
                    Ok(TypeName::Date)
                } else if name.eq_ignore_ascii_case("Ptr") || name.eq_ignore_ascii_case("LongPtr") {
                    Ok(TypeName::Ptr)
                } else if name.eq_ignore_ascii_case("LongLong") {
                    Ok(TypeName::Int64)
                } else if name.eq_ignore_ascii_case("FuncPtr") {
                    Ok(TypeName::FuncPtr)
                } else if name.eq_ignore_ascii_case("Object") {
                    Ok(TypeName::User("Object".to_string()))
                } else {
                    if self.match_simple(&TokenKind::Dot) {
                        let member =
                            self.expect_identifier("Expected type name after module qualifier")?;
                        name.push('.');
                        name.push_str(&member);
                    }
                    let ty = if self.check_simple(&TokenKind::LeftParen)
                        && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                    {
                        self.parse_generic_type_instance(name)?
                    } else {
                        TypeName::User(name)
                    };
                    Ok(ty)
                }
            }
            TokenKind::Any => Ok(TypeName::Variant),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Expected type name",
                Some(token.span),
            )),
        }?;
        if self.check_simple(&TokenKind::LeftBracket) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "Square-bracket array type syntax is not supported; use 'Dim name() As Type'",
                Some(self.peek().span),
            ));
        }
        Ok(ty)
    }

    pub(super) fn parse_optional_type_params(&mut self) -> Result<Vec<String>, Diagnostic> {
        if !(self.check_simple(&TokenKind::LeftParen)
            && matches!(self.peek_next_kind(), Some(TokenKind::Of)))
        {
            return Ok(Vec::new());
        }
        self.expect_simple(TokenKind::LeftParen, "Expected '(' before type parameters")?;
        self.expect_simple(TokenKind::Of, "Expected 'Of' in type parameter list")?;
        let mut params = Vec::new();
        loop {
            let name = self.expect_identifier("Expected type parameter name")?;
            if params
                .iter()
                .any(|param: &String| param.eq_ignore_ascii_case(&name))
            {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!("Type parameter '{}' is already declared", name),
                    Some(self.previous().span),
                ));
            }
            params.push(name);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "Expected ')' after type parameters")?;
        Ok(params)
    }

    pub(super) fn parse_optional_type_args(&mut self) -> Result<Vec<TypeName>, Diagnostic> {
        if !(self.check_simple(&TokenKind::LeftParen)
            && matches!(self.peek_next_kind(), Some(TokenKind::Of)))
        {
            return Ok(Vec::new());
        }
        self.expect_simple(TokenKind::LeftParen, "Expected '(' before type arguments")?;
        self.expect_simple(TokenKind::Of, "Expected 'Of' in type argument list")?;
        let args = self.parse_type_argument_tail()?;
        Ok(args)
    }

    pub(super) fn parse_generic_type_instance(
        &mut self,
        name: String,
    ) -> Result<TypeName, Diagnostic> {
        self.expect_simple(TokenKind::LeftParen, "Expected '(' before type arguments")?;
        self.expect_simple(TokenKind::Of, "Expected 'Of' in generic type")?;
        let args = self.parse_type_argument_tail()?;
        Ok(TypeName::GenericInstance { name, args })
    }

    fn parse_type_argument_tail(&mut self) -> Result<Vec<TypeName>, Diagnostic> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type_name()?);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "Expected ')' after type arguments")?;
        Ok(args)
    }
    pub(super) fn apply_class_attribute(
        &self,
        attribute: &AttributeDecl,
        members: &mut [ClassMember],
    ) {
        if !attribute.name.eq_ignore_ascii_case("VB_UserMemId") {
            return;
        }
        let is_default_member = attribute.value == "0";
        let is_enumerator = attribute.value == "-4";
        if !is_default_member && !is_enumerator {
            return;
        }
        let member_name = attribute.target.as_str();
        for member in members.iter_mut().rev() {
            match member {
                ClassMember::Property(property)
                    if property.name.eq_ignore_ascii_case(member_name) && is_default_member =>
                {
                    property.is_default = true;
                    return;
                }
                ClassMember::Property(property)
                    if property.name.eq_ignore_ascii_case(member_name) && is_enumerator =>
                {
                    property.is_enumerator = true;
                    return;
                }
                ClassMember::Function(function)
                    if function.function.name.eq_ignore_ascii_case(member_name)
                        && is_enumerator =>
                {
                    function.is_enumerator = true;
                    return;
                }
                _ => {}
            }
        }
    }
}
