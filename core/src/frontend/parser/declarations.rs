use super::*;
use crate::runtime::{Diagnostic, Span, TypeName};

#[derive(Clone)]
struct VbNetPropertyHeader {
    visibility: Visibility,
    is_default: bool,
    is_readonly: bool,
    is_writeonly: bool,
    start: Span,
    name: String,
    params: Vec<Parameter>,
    ty: TypeName,
}

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
            .expect_simple(TokenKind::Imports, "Expected 'Imports'")?
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
                    "Expected module name or string literal after 'Imports'",
                    Some(token.span),
                ));
            }
        };

        let alias = if self.match_simple(&TokenKind::As) {
            Some(self.expect_identifier("Expected import alias after 'As'")?)
        } else if self.match_simple(&TokenKind::Equal) {
            let alias = Some(module);
            let mut module = self.expect_identifier("Expected module name after '='")?;
            while self.match_simple(&TokenKind::Dot) {
                let next =
                    self.expect_identifier("Expected identifier after '.' in import path")?;
                module.push('.');
                module.push_str(&next);
            }
            return Ok(ImportDecl {
                module,
                alias,
                span: Span::new(self.file_id, start.start, self.previous().span.end),
            });
        } else {
            None
        };
        let end = self.previous().span;
        self.expect_statement_end("Expected newline after Imports declaration")?;
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
        self.expect_statement_end("Expected newline after Class declaration")?;

        let mut members = Vec::new();
        let mut attributes = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndClass]) {
            if self.match_simple(&TokenKind::Implements) {
                loop {
                    implements.push(self.parse_type_name()?);
                    if !self.match_simple(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_statement_end("Expected newline after Implements")?;
                self.skip_newlines();
                continue;
            }

            if matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute"))
            {
                let attribute = self.parse_attribute_decl()?;
                self.apply_class_attribute(&attribute, &mut members);
                attributes.push(attribute);
            } else {
                members.extend(self.parse_class_member()?);
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
            generic_constraints,
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
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
            generic_constraints,
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

    pub(super) fn parse_class_member(&mut self) -> Result<Vec<ClassMember>, Diagnostic> {
        self.skip_newlines();
        let modern_attributes = self.parse_modern_attributes()?;
        let explicit_visibility = self.parse_optional_visibility();
        let visibility = explicit_visibility.unwrap_or(Visibility::Public);
        let override_kind = self.parse_optional_override_kind();
        let is_shared = self.match_simple(&TokenKind::Shared);
        let is_async = self.match_simple(&TokenKind::Async);
        let is_readonly = self.match_simple(&TokenKind::ReadOnly);
        let is_writeonly = self.match_simple(&TokenKind::WriteOnly);
        if is_readonly && is_writeonly {
            return Err(self.error_here("Property cannot be both ReadOnly and WriteOnly"));
        }
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
                self.parse_event(visibility)
                    .map(ClassMember::Event)
                    .map(|member| vec![member])
            }
            TokenKind::Const => self.parse_module_consts(visibility).and_then(|mut consts| {
                if consts.len() == 1 {
                    Ok(vec![ClassMember::Const(consts.remove(0))])
                } else {
                    Err(self.error_here("Class Const declarations must be one per member"))
                }
            }),
            TokenKind::Sub => {
                if is_iterator {
                    return Err(self.error_here("Iterator is not supported on Sub"));
                }
                if matches!(self.peek_next_kind(), Some(TokenKind::New)) {
                    Ok(vec![ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind,
                        is_shared,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "New",
                            "Initialize",
                            modern_attributes,
                            is_async,
                        )?,
                    })])
                } else if matches!(
                    self.peek_next_kind(),
                    Some(TokenKind::Identifier(name, _)) if name.eq_ignore_ascii_case("Terminate")
                ) {
                    Ok(vec![ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind,
                        is_shared,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "Terminate",
                            "Terminate",
                            modern_attributes,
                            is_async,
                        )?,
                    })])
                } else {
                    let (procedure, implements) =
                        self.parse_class_procedure(visibility, modern_attributes, is_async)?;
                    Ok(vec![ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind,
                        is_shared,
                        implements,
                        procedure,
                    })])
                }
            }
            TokenKind::Function => {
                let mut function = self.parse_class_function(
                    visibility,
                    is_iterator,
                    modern_attributes,
                    is_async,
                )?;
                function.override_kind = override_kind;
                function.is_shared = is_shared;
                Ok(vec![ClassMember::Function(function)])
            }
            TokenKind::Property => {
                let mut members = self.parse_class_property_members(
                    visibility,
                    is_default,
                    is_iterator,
                    is_readonly,
                    is_writeonly,
                )?;
                for member in &mut members {
                    if let ClassMember::Property(property) = member {
                        property.override_kind = override_kind;
                        property.is_shared = is_shared;
                    } else if let ClassMember::Field(field) = member {
                        field.is_shared = is_shared;
                    }
                }
                Ok(members)
            }
            TokenKind::Operator => {
                if !is_shared {
                    return Err(self.error_here("Operators must be declared as Shared"));
                }
                self.parse_operator_decl(visibility)
                    .map(ClassMember::Operator)
                    .map(|member| vec![member])
            }
            TokenKind::Type | TokenKind::Structure => self
                .parse_type_decl(visibility)
                .map(ClassMember::Type)
                .map(|member| vec![member]),
            TokenKind::Class => self
                .parse_class_decl(visibility, ClassInheritance::Normal)
                .map(|member| ClassMember::Class(Box::new(member)))
                .map(|member| vec![member]),
            TokenKind::Enum => self
                .parse_enum_decl(visibility)
                .map(ClassMember::Enum)
                .map(|member| vec![member]),
            TokenKind::Declare => self
                .parse_declare_decl(visibility)
                .map(ClassMember::Declare)
                .map(|member| vec![member]),
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
                    Ok(vec![ClassMember::Field(
                        fields.into_iter().next().expect("len checked"),
                    )])
                } else {
                    Ok(vec![ClassMember::Fields(fields)])
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
            let mut interface_name = self.parse_type_name()?;
            let member_name = if self.match_simple(&TokenKind::Dot) {
                self.expect_identifier("Expected interface member name")?
            } else {
                match interface_name {
                    TypeName::User(ref mut name) if name.contains('.') => {
                        let (new_name, member) = name.rsplit_once('.').unwrap();
                        let member = member.to_string();
                        *name = new_name.to_string();
                        member
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "Expected '.' in Implements clause",
                            Some(self.previous().span),
                        ));
                    }
                }
            };
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
        self.parse_optional_where_clauses()?;
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
            is_readonly: false,
            is_writeonly: false,
            name,
            kind,
            params,
            return_type,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_class_property_members(
        &mut self,
        visibility: Visibility,
        is_default: bool,
        is_iterator: bool,
        is_readonly: bool,
        is_writeonly: bool,
    ) -> Result<Vec<ClassMember>, Diagnostic> {
        if matches!(
            self.peek_next_kind(),
            Some(TokenKind::Get | TokenKind::Let | TokenKind::Set)
        ) {
            let mut property = self.parse_property(visibility, is_default, is_iterator)?;
            match property.kind {
                PropertyKind::Get if is_writeonly => {
                    return Err(self.error_here("WriteOnly property cannot declare a Get accessor"));
                }
                PropertyKind::Let | PropertyKind::Set if is_readonly => {
                    return Err(
                        self.error_here("ReadOnly property cannot declare a Let or Set accessor")
                    );
                }
                _ => {}
            }
            property.is_readonly = is_readonly;
            property.is_writeonly = is_writeonly;
            return Ok(vec![ClassMember::Property(property)]);
        }

        self.parse_vbnet_property_members(visibility, is_default, is_readonly, is_writeonly)
    }

    fn parse_vbnet_property_members(
        &mut self,
        visibility: Visibility,
        is_default: bool,
        is_readonly: bool,
        is_writeonly: bool,
    ) -> Result<Vec<ClassMember>, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Property, "Expected 'Property'")?
            .span;
        let name = self.expect_identifier("Expected property name")?;
        let params = if self.match_simple(&TokenKind::LeftParen) {
            let params = self.parse_parameters()?;
            self.expect_simple(
                TokenKind::RightParen,
                "Expected ')' after property parameters",
            )?;
            params
        } else {
            Vec::new()
        };
        self.expect_simple(TokenKind::As, "Expected 'As' after property name")?;
        let ty = self.parse_type_name()?;
        let initializer = if self.match_simple(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect_statement_end("Expected newline after property declaration")?;

        if matches!(self.peek_kind(), TokenKind::Get | TokenKind::Set) {
            if initializer.is_some() {
                return Err(self.error_here("Block properties cannot use an initializer"));
            }
            return self.parse_vbnet_block_property(VbNetPropertyHeader {
                visibility,
                is_default,
                is_readonly,
                is_writeonly,
                start,
                name,
                params,
                ty,
            });
        }

        if !params.is_empty() {
            return Err(self.error_here("Auto properties cannot declare parameters"));
        }
        self.lower_auto_property(
            VbNetPropertyHeader {
                visibility,
                is_default,
                is_readonly,
                is_writeonly,
                start,
                name,
                params,
                ty,
            },
            initializer,
        )
    }

    fn parse_vbnet_block_property(
        &mut self,
        header: VbNetPropertyHeader,
    ) -> Result<Vec<ClassMember>, Diagnostic> {
        let mut members = Vec::new();
        let mut saw_get = false;
        let mut saw_set = false;

        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndProperty]) {
            self.skip_newlines();
            if self.matches_block_end(&[BlockEnd::EndProperty]) {
                break;
            }
            match self.peek_kind() {
                TokenKind::Get => {
                    if header.is_writeonly {
                        return Err(
                            self.error_here("WriteOnly property cannot declare a Get accessor")
                        );
                    }
                    if saw_get {
                        return Err(self.error_here("Property Get accessor is already declared"));
                    }
                    let body = self.parse_vbnet_get_body()?;
                    members.push(ClassMember::Property(ClassProperty {
                        visibility: header.visibility,
                        override_kind: OverrideKind::None,
                        is_shared: false,
                        implements: Vec::new(),
                        is_default: header.is_default,
                        is_enumerator: false,
                        is_iterator: false,
                        is_readonly: header.is_readonly,
                        is_writeonly: header.is_writeonly,
                        name: header.name.clone(),
                        kind: PropertyKind::Get,
                        params: header.params.clone(),
                        return_type: Some(header.ty.clone()),
                        body,
                        span: header.start,
                    }));
                    saw_get = true;
                }
                TokenKind::Set => {
                    if header.is_readonly {
                        return Err(
                            self.error_here("ReadOnly property cannot declare a Set accessor")
                        );
                    }
                    if saw_set {
                        return Err(self.error_here("Property Set accessor is already declared"));
                    }
                    let (set_params, body) = self.parse_vbnet_set_body(&header.ty)?;
                    let mut setter_params = header.params.clone();
                    setter_params.extend(set_params);
                    members.push(ClassMember::Property(ClassProperty {
                        visibility: header.visibility,
                        override_kind: OverrideKind::None,
                        is_shared: false,
                        implements: Vec::new(),
                        is_default: false,
                        is_enumerator: false,
                        is_iterator: false,
                        is_readonly: header.is_readonly,
                        is_writeonly: header.is_writeonly,
                        name: header.name.clone(),
                        kind: PropertyKind::Let,
                        params: setter_params,
                        return_type: None,
                        body,
                        span: header.start,
                    }));
                    saw_set = true;
                }
                _ => return Err(self.error_here("Expected Get, Set, or End Property")),
            }
        }

        self.expect_simple(TokenKind::End, "Expected 'End Property'")?;
        self.expect_simple(TokenKind::Property, "Expected 'Property' after 'End'")?;
        self.consume_statement_end();

        if members.is_empty() {
            return Err(self.error_here("Property must declare at least one accessor"));
        }
        Ok(members)
    }

    fn parse_vbnet_get_body(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect_simple(TokenKind::Get, "Expected 'Get'")?;
        self.expect_statement_end("Expected newline after Get")?;
        let body = self.parse_block_until(&[BlockEnd::EndGet])?;
        self.expect_simple(TokenKind::End, "Expected 'End Get'")?;
        self.expect_simple(TokenKind::Get, "Expected 'Get' after 'End'")?;
        self.consume_statement_end();
        Ok(body)
    }

    fn parse_vbnet_set_body(
        &mut self,
        ty: &TypeName,
    ) -> Result<(Vec<Parameter>, Vec<Stmt>), Diagnostic> {
        let start = self.expect_simple(TokenKind::Set, "Expected 'Set'")?.span;
        let params = if self.match_simple(&TokenKind::LeftParen) {
            let params = self.parse_parameters()?;
            self.expect_simple(TokenKind::RightParen, "Expected ')' after Set parameter")?;
            params
        } else {
            vec![Parameter {
                name: "value".to_string(),
                ty: ty.clone(),
                mode: PassingMode::ByVal,
                is_optional: false,
                optional_default: None,
                is_param_array: false,
                span: start,
            }]
        };
        self.expect_statement_end("Expected newline after Set")?;
        let body = self.parse_block_until(&[BlockEnd::EndSet])?;
        self.expect_simple(TokenKind::End, "Expected 'End Set'")?;
        self.expect_simple(TokenKind::Set, "Expected 'Set' after 'End'")?;
        self.consume_statement_end();
        Ok((params, body))
    }

    fn lower_auto_property(
        &mut self,
        header: VbNetPropertyHeader,
        initializer: Option<Expr>,
    ) -> Result<Vec<ClassMember>, Diagnostic> {
        let backing_name = format!("__valo_auto_property_{}", header.name);
        let field = ClassMember::Field(ClassField {
            visibility: Visibility::Private,
            is_shared: false,
            with_events: false,
            name: backing_name.clone(),
            ty: Some(header.ty.clone()),
            array: None,
            initializer,
            span: header.start,
        });
        let mut members = vec![field];
        if !header.is_writeonly {
            members.push(ClassMember::Property(ClassProperty {
                visibility: header.visibility,
                override_kind: OverrideKind::None,
                is_shared: false,
                implements: Vec::new(),
                is_default: header.is_default,
                is_enumerator: false,
                is_iterator: false,
                is_readonly: header.is_readonly,
                is_writeonly: header.is_writeonly,
                name: header.name.clone(),
                kind: PropertyKind::Get,
                params: Vec::new(),
                return_type: Some(header.ty.clone()),
                body: vec![Stmt::Return {
                    expr: Expr {
                        kind: ExprKind::Variable(backing_name.clone()),
                        span: header.start,
                    },
                    span: header.start,
                }],
                span: header.start,
            }));
        }
        if !header.is_readonly {
            members.push(ClassMember::Property(ClassProperty {
                visibility: header.visibility,
                override_kind: OverrideKind::None,
                is_shared: false,
                implements: Vec::new(),
                is_default: false,
                is_enumerator: false,
                is_iterator: false,
                is_readonly: header.is_readonly,
                is_writeonly: header.is_writeonly,
                name: header.name,
                kind: PropertyKind::Let,
                params: vec![Parameter {
                    name: "value".to_string(),
                    ty: header.ty,
                    mode: PassingMode::ByVal,
                    is_optional: false,
                    optional_default: None,
                    is_param_array: false,
                    span: header.start,
                }],
                return_type: None,
                body: vec![Stmt::Assign {
                    target: AssignTarget::Variable {
                        name: backing_name,
                        span: header.start,
                    },
                    expr: Expr {
                        kind: ExprKind::Variable("value".to_string()),
                        span: header.start,
                    },
                    span: header.start,
                }],
                span: header.start,
            }));
        }
        Ok(members)
    }

    pub(super) fn parse_modern_attributes(
        &mut self,
    ) -> Result<Vec<crate::frontend::ast::ModernAttribute>, Diagnostic> {
        let mut attributes = Vec::new();
        while self.match_simple(&TokenKind::Less) {
            loop {
                let name_start = self.peek().span;
                let mut name = self.expect_identifier("Expected attribute name")?;
                while self.match_simple(&TokenKind::Dot) {
                    name.push('.');
                    name.push_str(&self.expect_identifier("Expected attribute name after '.'")?);
                }

                let mut args = None;
                if self.match_simple(&TokenKind::LeftParen) {
                    if !self.check_simple(&TokenKind::RightParen) {
                        let mut arg_list = Vec::new();
                        loop {
                            arg_list.push(self.parse_expression()?);
                            if !self.match_simple(&TokenKind::Comma) {
                                break;
                            }
                        }
                        args = Some(arg_list);
                    } else {
                        args = Some(Vec::new());
                    }
                    self.expect_simple(
                        TokenKind::RightParen,
                        "Expected ')' after attribute arguments",
                    )?;
                }

                let span = Span::new(self.file_id, name_start.start, self.previous().span.end);
                attributes.push(crate::frontend::ast::ModernAttribute { name, args, span });

                if self.match_simple(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
            self.expect_simple(TokenKind::Greater, "Expected '>' after attribute list")?;
            self.skip_newlines();
        }
        Ok(attributes)
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
        self.expect_statement_end(&format!("Expected newline after {keyword} declaration"))?;

        let mut fields = Vec::new();
        let mut members = Vec::new();
        let mut implements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[end_block]) {
            if self.match_simple(&TokenKind::Implements) {
                loop {
                    implements.push(self.parse_type_name()?);
                    if !self.match_simple(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_statement_end("Expected newline after Implements")?;
                self.skip_newlines();
                continue;
            }

            if kind == TypeKind::Type && self.type_member_starts_non_field() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    "Type declarations support fields only; use Structure for methods and properties",
                    Some(self.peek().span),
                ));
            }

            if kind == TypeKind::Structure && self.structure_member_starts_non_field() {
                members.extend(self.parse_structure_member()?);
            } else {
                let explicit_visibility = self.parse_optional_visibility();
                if kind == TypeKind::Type && explicit_visibility.is_some() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        "VBA Type fields cannot use Public or Private",
                        Some(self.previous().span),
                    ));
                }
                if self.match_simple(&TokenKind::WithEvents) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        "Structure fields cannot use WithEvents",
                        Some(self.previous().span),
                    ));
                }
                let has_dim = self.match_simple(&TokenKind::Dim);
                if kind == TypeKind::Type && has_dim {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        "VBA Type fields cannot use Dim",
                        Some(self.previous().span),
                    ));
                }
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
            generic_constraints,
            implements,
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
            | TokenKind::Shared
            | TokenKind::Operator
            | TokenKind::Event => true,
            TokenKind::Public | TokenKind::Private => matches!(
                self.peek_next_kind(),
                Some(
                    TokenKind::Sub
                        | TokenKind::Function
                        | TokenKind::Iterator
                        | TokenKind::Property
                        | TokenKind::Default
                        | TokenKind::Shared
                        | TokenKind::Operator
                        | TokenKind::Event
                )
            ),
            _ => false,
        }
    }

    fn parse_structure_member(&mut self) -> Result<Vec<ClassMember>, Diagnostic> {
        self.skip_newlines();
        let modern_attributes = self.parse_modern_attributes()?;
        let explicit_visibility = self.parse_optional_visibility();
        let visibility = explicit_visibility.unwrap_or(Visibility::Public);
        let is_shared = self.match_simple(&TokenKind::Shared);
        let is_async = self.match_simple(&TokenKind::Async);
        let is_readonly = self.match_simple(&TokenKind::ReadOnly);
        let is_writeonly = self.match_simple(&TokenKind::WriteOnly);
        if is_readonly && is_writeonly {
            return Err(self.error_here("Property cannot be both ReadOnly and WriteOnly"));
        }
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
                    Ok(vec![ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared,
                        implements: Vec::new(),
                        procedure: self.parse_lifecycle_sub_procedure(
                            visibility,
                            "New",
                            "Initialize",
                            modern_attributes,
                            is_async,
                        )?,
                    })])
                } else {
                    let (procedure, implements) =
                        self.parse_class_procedure(visibility, modern_attributes, is_async)?;
                    Ok(vec![ClassMember::Sub(ClassSub {
                        visibility,
                        override_kind: OverrideKind::None,
                        is_shared,
                        implements,
                        procedure,
                    })])
                }
            }
            TokenKind::Function => {
                let mut function =
                    self.parse_class_function(visibility, false, modern_attributes, is_async)?;
                function.is_shared = is_shared;
                Ok(vec![ClassMember::Function(function)])
            }
            TokenKind::Property => {
                let mut members = self.parse_class_property_members(
                    visibility,
                    is_default,
                    false, // is_iterator
                    is_readonly,
                    is_writeonly,
                )?;
                for member in &mut members {
                    if let ClassMember::Property(property) = member {
                        property.is_shared = is_shared;
                    } else if let ClassMember::Field(field) = member {
                        field.is_shared = is_shared;
                    }
                }
                Ok(members)
            }
            TokenKind::Operator => {
                if !is_shared {
                    return Err(self.error_here("Operators must be declared as Shared"));
                }
                self.parse_operator_decl(visibility)
                    .map(ClassMember::Operator)
                    .map(|member| vec![member])
            }
            _ if is_default => Err(self.error_here("Default is only supported on Property")),
            _ => Err(self.error_here("Expected structure member")),
        }
    }

    pub(super) fn parse_procedure(
        &mut self,
        visibility: Visibility,
        attributes: Vec<crate::frontend::ast::ModernAttribute>,
        is_async: bool,
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
        self.expect_statement_end("Expected newline after procedure declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndSub])?;
        self.expect_simple(TokenKind::End, "Expected 'End Sub'")?;
        let end = self
            .expect_simple(TokenKind::Sub, "Expected 'Sub' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Procedure {
            attributes,
            visibility,
            is_async,
            name,
            type_params,
            generic_constraints,
            params,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_class_procedure(
        &mut self,
        visibility: Visibility,
        attributes: Vec<crate::frontend::ast::ModernAttribute>,
        is_async: bool,
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
        self.expect_statement_end("Expected newline after procedure declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndSub])?;
        self.expect_simple(TokenKind::End, "Expected 'End Sub'")?;
        let end = self
            .expect_simple(TokenKind::Sub, "Expected 'Sub' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok((
            Procedure {
                attributes,
                visibility,
                is_async,
                name,
                type_params,
                generic_constraints,
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
        attributes: Vec<crate::frontend::ast::ModernAttribute>,
        is_async: bool,
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
            attributes,
            visibility,
            is_async,
            name: canonical_name.to_string(),
            type_params: Vec::new(),
            generic_constraints: Vec::new(),
            params,
            body,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    pub(super) fn parse_function(
        &mut self,
        visibility: Visibility,
        is_iterator: bool,
        attributes: Vec<crate::frontend::ast::ModernAttribute>,
        is_async: bool,
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
        self.expect_statement_end("Expected newline after function declaration")?;

        let body = self.parse_block_until(&[BlockEnd::EndFunction])?;
        self.expect_simple(TokenKind::End, "Expected 'End Function'")?;
        let end = self
            .expect_simple(TokenKind::Function, "Expected 'Function' after 'End'")?
            .span;
        self.consume_statement_end();

        Ok(Function {
            attributes,
            visibility,
            is_async,
            name,
            is_iterator,
            type_params,
            generic_constraints,
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
        attributes: Vec<crate::frontend::ast::ModernAttribute>,
        is_async: bool,
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
        self.parse_optional_where_clauses()?;
        let generic_constraints = self.take_generic_constraints();
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
                attributes,
                visibility,
                is_async,
                name,
                is_iterator,
                type_params,
                generic_constraints,
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
            if self.at_statement_separator() {
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
            TokenKind::Function => Some(("Function".to_string(), None)),
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
            TokenKind::Error => Ok(TypeName::User("Error".to_string())),
            TokenKind::Collection => Ok(TypeName::User("Collection".to_string())),
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
                    Ok(TypeName::Variant)
                } else if name.eq_ignore_ascii_case("Collection") {
                    Ok(TypeName::User("Collection".to_string()))
                } else {
                    while self.match_simple(&TokenKind::Dot) {
                        let member =
                            self.expect_identifier("Expected type or namespace name after '.'")?;
                        name.push('.');
                        name.push_str(&member);
                    }
                    let mut ty = if self.check_simple(&TokenKind::LeftParen)
                        && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                    {
                        self.parse_generic_type_instance(name)?
                    } else {
                        TypeName::User(name)
                    };
                    while self.match_simple(&TokenKind::Dot) {
                        let member = self.expect_identifier("Expected member name after '.'")?;
                        let mut base_name = ty.base_user_name().unwrap().to_string();
                        base_name.push('.');
                        base_name.push_str(&member);
                        let mut args = match ty {
                            TypeName::GenericInstance { args, .. } => args,
                            _ => Vec::new(),
                        };
                        if self.check_simple(&TokenKind::LeftParen)
                            && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                        {
                            let inner_ty = self.parse_generic_type_instance(base_name.clone())?;
                            if let TypeName::GenericInstance {
                                args: mut inner_args,
                                ..
                            } = inner_ty
                            {
                                args.append(&mut inner_args);
                            }
                        }
                        if args.is_empty() {
                            ty = TypeName::User(base_name);
                        } else {
                            ty = TypeName::GenericInstance {
                                name: base_name,
                                args,
                            };
                        }
                    }
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

        if self.match_simple(&TokenKind::Question) {
            Ok(TypeName::Nullable(Box::new(ty)))
        } else {
            Ok(ty)
        }
    }

    pub(super) fn parse_optional_type_params(&mut self) -> Result<Vec<String>, Diagnostic> {
        self.pending_generic_constraints.clear();
        if !(self.check_simple(&TokenKind::LeftParen)
            && matches!(self.peek_next_kind(), Some(TokenKind::Of)))
        {
            return Ok(Vec::new());
        }
        self.expect_simple(TokenKind::LeftParen, "Expected '(' before type parameters")?;
        self.expect_simple(TokenKind::Of, "Expected 'Of' in type parameter list")?;
        let mut params = Vec::new();
        loop {
            self.parse_optional_type_param_variance();
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
            if self.match_simple(&TokenKind::As) {
                let constraint = self.parse_type_param_constraint(name.clone())?;
                self.merge_generic_constraint(constraint);
            }
            params.push(name);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "Expected ')' after type parameters")?;
        Ok(params)
    }

    fn parse_optional_type_param_variance(&mut self) {
        if self.match_simple(&TokenKind::In) {
            return;
        }
        if matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Out"))
        {
            self.advance();
        }
    }

    fn parse_type_param_constraint(
        &mut self,
        name: String,
    ) -> Result<GenericParamConstraint, Diagnostic> {
        let mut constraint = GenericParamConstraint {
            name,
            ..GenericParamConstraint::default()
        };
        if self.match_simple(&TokenKind::LeftBrace) {
            loop {
                self.parse_single_type_param_constraint(&mut constraint)?;
                if self.match_simple(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
            self.expect_simple(
                TokenKind::RightBrace,
                "Expected '}' after type parameter constraints",
            )?;
            return Ok(constraint);
        }

        self.parse_single_type_param_constraint(&mut constraint)?;
        Ok(constraint)
    }

    fn parse_single_type_param_constraint(
        &mut self,
        constraint: &mut GenericParamConstraint,
    ) -> Result<(), Diagnostic> {
        if self.match_simple(&TokenKind::Class) {
            constraint.require_class = true;
            return Ok(());
        }
        if self.match_simple(&TokenKind::Structure) {
            constraint.require_structure = true;
            return Ok(());
        }
        if self.match_simple(&TokenKind::New) {
            if self.match_simple(&TokenKind::LeftParen) {
                self.expect_simple(TokenKind::RightParen, "Expected ')' after New constraint")?;
            }
            constraint.require_new = true;
            return Ok(());
        }

        let bound = self.parse_type_name()?;
        constraint.bounds.push(bound);
        Ok(())
    }

    fn parse_operator_decl(&mut self, visibility: Visibility) -> Result<OperatorDecl, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Operator, "Expected 'Operator'")?
            .span;
        let mut kind = match self.peek_kind() {
            TokenKind::Plus => {
                self.advance();
                OperatorKind::Add
            }
            TokenKind::Minus => {
                self.advance();
                OperatorKind::Subtract
            }
            TokenKind::Star => {
                self.advance();
                OperatorKind::Multiply
            }
            TokenKind::Slash => {
                self.advance();
                OperatorKind::Divide
            }
            TokenKind::Backslash => {
                self.advance();
                OperatorKind::IntegerDivide
            }
            TokenKind::Caret => {
                self.advance();
                OperatorKind::Exponent
            }
            TokenKind::Mod => {
                self.advance();
                OperatorKind::Modulo
            }
            TokenKind::And => {
                self.advance();
                OperatorKind::And
            }
            TokenKind::Or => {
                self.advance();
                OperatorKind::Or
            }
            TokenKind::Xor => {
                self.advance();
                OperatorKind::Xor
            }
            TokenKind::Not => {
                self.advance();
                OperatorKind::Not
            }
            TokenKind::Equal => {
                self.advance();
                OperatorKind::Equal
            }
            TokenKind::NotEqual => {
                self.advance();
                OperatorKind::NotEqual
            }
            TokenKind::Less => {
                self.advance();
                OperatorKind::Less
            }
            TokenKind::Greater => {
                self.advance();
                OperatorKind::Greater
            }
            TokenKind::LessEqual => {
                self.advance();
                OperatorKind::LessEqual
            }
            TokenKind::GreaterEqual => {
                self.advance();
                OperatorKind::GreaterEqual
            }
            TokenKind::Like => {
                self.advance();
                OperatorKind::Like
            }
            TokenKind::Ampersand => {
                self.advance();
                OperatorKind::Concatenate
            }
            TokenKind::True => {
                self.advance();
                OperatorKind::True
            }
            TokenKind::False => {
                self.advance();
                OperatorKind::False
            }
            _ => {
                return Err(self.error_here("Expected operator symbol after 'Operator'"));
            }
        };

        self.expect_simple(TokenKind::LeftParen, "Expected '(' after operator symbol")?;
        let params = self.parse_parameters()?;
        self.expect_simple(
            TokenKind::RightParen,
            "Expected ')' after operator parameters",
        )?;

        // Disambiguate Add/Subtract based on param count
        if params.len() == 1 {
            if kind == OperatorKind::Add {
                kind = OperatorKind::UnaryPlus;
            } else if kind == OperatorKind::Subtract {
                kind = OperatorKind::UnaryMinus;
            }
        }
        self.expect_simple(TokenKind::As, "Expected 'As' after operator parameters")?;
        let return_type = self.parse_type_name()?;
        self.expect_statement_end("Expected newline after Operator header")?;

        let body = self.parse_block_until(&[BlockEnd::EndOperator])?;
        let end = self
            .expect_simple(TokenKind::End, "Expected 'End Operator'")?
            .span;
        self.expect_simple(TokenKind::Operator, "Expected 'Operator' after 'End'")?;
        self.consume_statement_end();

        Ok(OperatorDecl {
            visibility,
            kind,
            params,
            return_type,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_optional_where_clauses(&mut self) -> Result<(), Diagnostic> {
        while matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Where"))
        {
            self.advance();
            let name = self.expect_identifier("Expected type parameter name after 'Where'")?;
            self.expect_simple(TokenKind::Colon, "Expected ':' after Where type parameter")?;
            let constraint = self.parse_type_param_constraint(name.clone())?;
            self.merge_generic_constraint(constraint);
            while self.match_simple(&TokenKind::Comma) {
                let constraint = self.parse_type_param_constraint(name.clone())?;
                self.merge_generic_constraint(constraint);
            }
        }
        Ok(())
    }

    fn merge_generic_constraint(&mut self, constraint: GenericParamConstraint) {
        let entry = self
            .pending_generic_constraints
            .entry(constraint.name.to_ascii_lowercase())
            .or_insert_with(|| GenericParamConstraint {
                name: constraint.name.clone(),
                ..GenericParamConstraint::default()
            });
        entry.require_class |= constraint.require_class;
        entry.require_structure |= constraint.require_structure;
        entry.require_new |= constraint.require_new;
        entry.bounds.extend(constraint.bounds);
    }

    fn take_generic_constraints(&mut self) -> Vec<GenericParamConstraint> {
        self.pending_generic_constraints
            .drain()
            .map(|(_, v)| v)
            .collect()
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
