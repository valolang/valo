use super::*;
use crate::runtime::{Diagnostic, Span};

impl Parser {
    pub(super) fn parse_block_until(&mut self, ends: &[BlockEnd]) -> Result<Vec<Stmt>, Diagnostic> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() && !self.matches_block_end(ends) {
            if self.matches_any_block_boundary() {
                break;
            }
            if matches!(self.peek_kind(), TokenKind::Integer(_)) {
                let token = self.advance();
                let TokenKind::Integer(number) = token.kind else {
                    unreachable!("peek checked");
                };
                statements.push(Stmt::Label {
                    name: number.to_string(),
                    span: token.span,
                });
                if self.check_simple(&TokenKind::Newline)
                    || self.check_simple(&TokenKind::Eof)
                    || self.matches_any_block_boundary()
                {
                    self.expect_statement_end("Expected newline after statement")?;
                    self.skip_newlines();
                    continue;
                }
            }
            statements.push(self.parse_stmt()?);
            if matches!(statements.last(), Some(Stmt::Label { .. }))
                && !self.at_statement_separator()
            {
                continue;
            }
            self.expect_statement_end("Expected newline after statement")?;
            self.skip_newlines();
        }

        Ok(statements)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        match self.peek_kind() {
            TokenKind::Dim => self.parse_dim(),
            TokenKind::Static => self.parse_static(),
            TokenKind::Const => self.parse_const_stmt(),
            TokenKind::If => self.parse_if(),
            TokenKind::Select => self.parse_select_case(),
            TokenKind::While => self.parse_while(),
            TokenKind::With => self.parse_with(),
            TokenKind::Using => self.parse_using(),
            TokenKind::Do => self.parse_do_loop(),
            TokenKind::For => self.parse_for(),
            TokenKind::GoTo => self.parse_goto(),
            TokenKind::Try => self.parse_try_catch(),
            TokenKind::Throw => self.parse_throw(),
            TokenKind::On => self.parse_on_error(),
            TokenKind::Resume => self.parse_resume(),
            TokenKind::Exit => self.parse_exit(),
            TokenKind::ReDim => self.parse_redim(),
            TokenKind::Erase => self.parse_erase(),
            TokenKind::LSet => self.parse_lset(),
            TokenKind::RSet => self.parse_rset(),
            TokenKind::RaiseEvent => self.parse_raise_event(),
            TokenKind::Let => self.parse_let_assignment(),
            TokenKind::Call => self.parse_call_statement(),
            TokenKind::Set => self.parse_set_assignment(),
            TokenKind::Console => self.parse_console_writeline(),
            TokenKind::End => self.parse_end_statement(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Yield => self.parse_yield(),
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Attribute") => {
                let _ = self.parse_attribute_decl()?;
                // Return a dummy statement that does nothing
                Ok(Stmt::Label {
                    name: String::new(),
                    span: self.previous().span,
                })
            }
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Open") => {
                self.parse_open_file()
            }
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Close") => {
                self.parse_close_file()
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Line")
                    && matches!(self.peek_next_kind(), Some(TokenKind::Identifier(next, _)) if next.eq_ignore_ascii_case("Input")) =>
            {
                self.parse_line_input()
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Input")
                    && matches!(self.peek_next_kind(), Some(TokenKind::Hash)) =>
            {
                self.parse_input_file()
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Print")
                    && matches!(self.peek_next_kind(), Some(TokenKind::Hash)) =>
            {
                self.parse_print_file()
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Write")
                    && matches!(self.peek_next_kind(), Some(TokenKind::Hash)) =>
            {
                self.parse_write_file()
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Get")
                    && matches!(
                        self.peek_next_kind(),
                        Some(TokenKind::Hash | TokenKind::Identifier(_, _) | TokenKind::Integer(_))
                    ) =>
            {
                self.parse_get_file()
            }
            TokenKind::Get
                if matches!(
                    self.peek_next_kind(),
                    Some(TokenKind::Hash | TokenKind::Identifier(_, _) | TokenKind::Integer(_))
                ) =>
            {
                self.parse_get_file()
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Put")
                    && matches!(
                        self.peek_next_kind(),
                        Some(TokenKind::Hash | TokenKind::Identifier(_, _) | TokenKind::Integer(_))
                    ) =>
            {
                self.parse_put_file()
            }
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Seek") => {
                if matches!(self.peek_next_kind(), Some(TokenKind::Hash)) {
                    self.parse_seek_file()
                } else {
                    self.parse_identifier_statement()
                }
            }
            TokenKind::Identifier(name, _)
                if name.eq_ignore_ascii_case("Name")
                    && !matches!(
                        self.peek_next_kind(),
                        Some(TokenKind::Equal | TokenKind::LeftParen | TokenKind::Dot)
                    ) =>
            {
                self.parse_name_file()
            }
            TokenKind::Identifier(_, _) | TokenKind::Me | TokenKind::Dot => {
                self.parse_identifier_statement()
            }
            TokenKind::Public | TokenKind::Private => {
                Err(self.error_here("Public/Private are only allowed inside Class"))
            }
            _ => Err(self.error_here("Expected statement")),
        }
    }

    fn parse_dim(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.advance().span;
        let decls = self.parse_variable_declarators("Dim")?;
        let end = decls.last().map(|decl| decl.span).unwrap_or(start);
        if decls.len() == 1 {
            let decl = decls.into_iter().next().expect("len checked");
            Ok(Stmt::Dim {
                name: decl.name,
                ty: decl.ty,
                array: decl.array,
                as_new: decl.as_new,
                new_args: decl.new_args,
                initializer: decl.initializer,
                span: Span::new(self.file_id, start.start, end.end),
            })
        } else {
            Ok(Stmt::DimMany {
                decls,
                span: Span::new(self.file_id, start.start, end.end),
            })
        }
    }

    fn parse_static(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.advance().span;
        let decls = self.parse_variable_declarators("Static")?;
        let end = decls.last().map(|decl| decl.span).unwrap_or(start);
        if decls.len() == 1 {
            let decl = decls.into_iter().next().expect("len checked");
            Ok(Stmt::Static {
                name: decl.name,
                ty: decl.ty,
                array: decl.array,
                as_new: decl.as_new,
                new_args: decl.new_args,
                initializer: decl.initializer,
                span: Span::new(self.file_id, start.start, end.end),
            })
        } else {
            Ok(Stmt::StaticMany {
                decls,
                span: Span::new(self.file_id, start.start, end.end),
            })
        }
    }

    pub(super) fn parse_variable_declarators(
        &mut self,
        keyword: &str,
    ) -> Result<Vec<VariableDecl>, Diagnostic> {
        let mut decls = Vec::new();
        loop {
            decls.push(self.parse_variable_declarator(keyword)?);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        Ok(decls)
    }

    pub(super) fn parse_variable_declarator(
        &mut self,
        keyword: &str,
    ) -> Result<VariableDecl, Diagnostic> {
        let start = self.peek().span;
        let token = self.advance();
        let (name, type_char) = match token.kind {
            TokenKind::Identifier(name, hint) => (name, hint),
            TokenKind::Version => ("VERSION".to_string(), None),
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Expected variable name after '{}'", keyword),
                    Some(token.span),
                )
                .with_primary_label(format!("Expected variable name after '{}'", keyword)));
            }
        };

        let array = if self.match_simple(&TokenKind::LeftParen) {
            if self.match_simple(&TokenKind::RightParen) {
                Some(ArrayDecl::Dynamic)
            } else {
                let mut bounds = Vec::new();
                loop {
                    let lower_or_size_token = self.advance();
                    let TokenKind::Integer(lower_or_size) = lower_or_size_token.kind else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Array size must be an Integer literal",
                            Some(lower_or_size_token.span),
                        ));
                    };
                    let bound = if self.match_simple(&TokenKind::To) {
                        let upper_token = self.advance();
                        let TokenKind::Integer(upper) = upper_token.kind else {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                "Array upper bound must be an Integer literal",
                                Some(upper_token.span),
                            ));
                        };
                        crate::runtime::ArrayBound {
                            lower: lower_or_size,
                            upper,
                        }
                    } else {
                        if lower_or_size < 0 {
                            return Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::ARRAY,
                                "Array size must be non-negative",
                                Some(lower_or_size_token.span),
                            ));
                        }
                        crate::runtime::ArrayBound {
                            lower: 0,
                            upper: lower_or_size,
                        }
                    };
                    if bound.upper < bound.lower {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Array upper bound must be greater than or equal to lower bound",
                            Some(lower_or_size_token.span),
                        ));
                    }
                    bounds.push(bound);
                    if !self.match_simple(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_simple(TokenKind::RightParen, "Expected ')' after array size")?;
                Some(ArrayDecl::Fixed(bounds))
            }
        } else {
            None
        };
        let mut as_new = false;
        let mut new_args = Vec::new();
        let as_ty = if self.match_simple(&TokenKind::As) {
            as_new = self.match_simple(&TokenKind::New);
            let ty = self.parse_type_name()?;
            if as_new && self.match_simple(&TokenKind::LeftParen) {
                new_args = self.finish_call_arguments()?;
            }
            Some(ty)
        } else {
            None
        };
        let ty = if let Some(as_ty) = as_ty {
            if let Some(type_char) = &type_char
                && !type_char.same_type(&as_ty)
            {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    format!(
                        "Type-declaration character implies {}, but As clause declares {}",
                        type_char.display_name(),
                        as_ty.display_name()
                    ),
                    Some(start),
                ));
            }
            Some(as_ty)
        } else {
            type_char.or(Some(crate::runtime::TypeName::Variant))
        };
        let initializer = if self.match_simple(&TokenKind::Equal) {
            if as_new {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "As New declarations cannot also use an '=' initializer",
                    Some(start),
                ));
            }
            Some(self.parse_expression()?)
        } else {
            None
        };
        let end = initializer
            .as_ref()
            .map(|expr| expr.span)
            .unwrap_or_else(|| self.previous().span);
        Ok(VariableDecl {
            name,
            ty,
            array,
            as_new,
            new_args,
            initializer,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_const_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.peek().span;
        let consts = self.parse_const_declarators(Visibility::Private)?;
        let end = consts.last().map(|decl| decl.span).unwrap_or(start);
        if consts.len() == 1 {
            let decl = consts.into_iter().next().expect("len checked");
            Ok(Stmt::Const {
                name: decl.name,
                ty: decl.ty,
                value: decl.value,
                span: Span::new(self.file_id, start.start, end.end),
            })
        } else {
            Ok(Stmt::ConstMany {
                consts,
                span: Span::new(self.file_id, start.start, end.end),
            })
        }
    }

    fn parse_open_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Open'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Open"));
        let path = self.parse_expression()?;
        self.expect_simple(TokenKind::For, "Expected 'For' in Open statement")?;
        let mode_token = self.advance();
        let mode = match mode_token.kind {
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Input") => OpenMode::Input,
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Output") => {
                OpenMode::Output
            }
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Append") => {
                OpenMode::Append
            }
            TokenKind::Binary => OpenMode::Binary,
            TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Random") => {
                OpenMode::Random
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Unsupported Open mode; supported modes are Input, Output, Append, Binary, and Random",
                    Some(mode_token.span),
                ));
            }
        };
        let mut access = None;
        let mut lock = None;
        let mut shared = false;
        loop {
            if self.match_identifier("Access") {
                if access.is_some() {
                    return Err(self.error_here("Open statement has duplicate Access clause"));
                }
                access = Some(self.parse_file_access("Access")?);
            } else if self.match_identifier("Lock") {
                if lock.is_some() || shared {
                    return Err(self.error_here(
                        "Open statement can use either Shared or one Lock clause, not both",
                    ));
                }
                lock = Some(self.parse_file_lock()?);
            } else if self.match_open_shared() {
                if lock.is_some() || shared {
                    return Err(self.error_here(
                        "Open statement can use either Shared or one Lock clause, not both",
                    ));
                }
                shared = true;
            } else {
                break;
            }
        }
        self.expect_simple(TokenKind::As, "Expected 'As' in Open statement")?;
        let number = self.parse_open_file_number_expr()?;
        let record_len = if self.match_identifier("Len") {
            self.expect_simple(TokenKind::Equal, "Expected '=' after Open Len")?;
            Some(self.parse_expression()?)
        } else {
            None
        };
        let end = record_len
            .as_ref()
            .map(|expr| expr.span)
            .unwrap_or(number.span);
        Ok(Stmt::OpenFile {
            path,
            mode,
            access,
            lock,
            shared,
            number,
            record_len,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn match_open_shared(&mut self) -> bool {
        if self.match_simple(&TokenKind::Shared) {
            true
        } else {
            self.match_identifier("Shared")
        }
    }

    fn parse_file_access(&mut self, clause: &str) -> Result<FileAccess, Diagnostic> {
        let first = self.expect_identifier(&format!("Expected access mode after '{clause}'"))?;
        let first_span = self.previous().span;
        if first.eq_ignore_ascii_case("Read") {
            if self.match_identifier("Write") {
                Ok(FileAccess::ReadWrite)
            } else {
                Ok(FileAccess::Read)
            }
        } else if first.eq_ignore_ascii_case("Write") {
            Ok(FileAccess::Write)
        } else {
            Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Expected Read, Write, or Read Write",
                Some(first_span),
            ))
        }
    }

    fn parse_file_lock(&mut self) -> Result<FileLock, Diagnostic> {
        match self.parse_file_access("Lock")? {
            FileAccess::Read => Ok(FileLock::Read),
            FileAccess::Write => Ok(FileLock::Write),
            FileAccess::ReadWrite => Ok(FileLock::ReadWrite),
        }
    }

    fn parse_close_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Close'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Close"));
        let mut numbers = Vec::new();
        if !self.at_statement_separator() {
            loop {
                numbers.push(self.parse_open_file_number_expr()?);
                if !self.match_simple(&TokenKind::Comma) {
                    break;
                }
            }
        }
        let end = numbers.last().map(|expr| expr.span).unwrap_or(start_span);
        Ok(Stmt::CloseFile {
            numbers,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_line_input(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Line'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Line"));
        let input = self.expect_identifier("Expected 'Input' after 'Line'")?;
        if !input.eq_ignore_ascii_case("Input") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Expected 'Input' after 'Line'",
                Some(self.previous().span),
            ));
        }
        let number = self.parse_open_file_number_expr()?;
        self.expect_simple(TokenKind::Comma, "Expected ',' after file number")?;
        let target = self.parse_assignment_target()?;
        let end = target.span();
        Ok(Stmt::LineInput {
            number,
            target,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_input_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Input'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Input"));
        let number = self.parse_open_file_number_expr()?;
        self.expect_simple(TokenKind::Comma, "Expected ',' after file number")?;
        let mut targets = Vec::new();
        loop {
            targets.push(self.parse_assignment_target()?);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        let end = targets
            .last()
            .map(AssignTarget::span)
            .unwrap_or(number.span);
        Ok(Stmt::InputFile {
            number,
            targets,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_print_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Print'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Print"));
        let number = self.parse_open_file_number_expr()?;
        let mut items = Vec::new();
        let mut trailing = None;
        if self.match_simple(&TokenKind::Comma) && !self.at_statement_separator() {
            let mut separator = PrintSeparator::None;
            loop {
                let expr = self.parse_expression()?;
                items.push(PrintItem { separator, expr });
                if self.match_simple(&TokenKind::Comma) {
                    if self.at_statement_separator() {
                        trailing = Some(PrintSeparator::Comma);
                        break;
                    }
                    separator = PrintSeparator::Comma;
                } else if self.match_simple(&TokenKind::Semicolon) {
                    if self.at_statement_separator() {
                        trailing = Some(PrintSeparator::Semicolon);
                        break;
                    }
                    separator = PrintSeparator::Semicolon;
                } else {
                    break;
                }
            }
        }
        let end = items
            .last()
            .map(|item| item.expr.span)
            .unwrap_or(number.span);
        Ok(Stmt::PrintFile {
            number,
            items,
            trailing,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_write_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Write'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Write"));
        let number = self.parse_file_number_expr()?;
        let mut args = Vec::new();
        if self.match_simple(&TokenKind::Comma) && !self.at_statement_separator() {
            loop {
                args.push(self.parse_expression()?);
                if !self.match_simple(&TokenKind::Comma) {
                    break;
                }
            }
        }
        let end = args.last().map(|arg| arg.span).unwrap_or(number.span);
        Ok(Stmt::WriteFile {
            number,
            args,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_get_file(&mut self) -> Result<Stmt, Diagnostic> {
        let token = self.advance();
        let start_span = self.previous().span;
        debug_assert!(
            matches!(token.kind, TokenKind::Get)
                || matches!(token.kind, TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case("Get"))
        );
        let number = self.parse_open_file_number_expr()?;
        self.expect_simple(TokenKind::Comma, "Expected ',' after file number")?;
        let position = if self.check_simple(&TokenKind::Comma) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect_simple(TokenKind::Comma, "Expected ',' before Get target")?;
        let target = self.parse_assignment_target()?;
        let end = target.span();
        Ok(Stmt::GetFile {
            number,
            position,
            target,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_put_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Put'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Put"));
        let number = self.parse_open_file_number_expr()?;
        self.expect_simple(TokenKind::Comma, "Expected ',' after file number")?;
        let position = if self.check_simple(&TokenKind::Comma) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect_simple(TokenKind::Comma, "Expected ',' before Put value")?;
        let expr = self.parse_expression()?;
        let end = expr.span;
        Ok(Stmt::PutFile {
            number,
            position,
            expr,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_seek_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Seek'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Seek"));
        let number = self.parse_file_number_expr()?;
        self.expect_simple(TokenKind::Comma, "Expected ',' after file number")?;
        let position = self.parse_expression()?;
        let end = position.span;
        Ok(Stmt::SeekFile {
            number,
            position,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_name_file(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_identifier("Expected 'Name'")?;
        let start_span = self.previous().span;
        debug_assert!(start.eq_ignore_ascii_case("Name"));
        let old_path = self.parse_expression()?;
        self.expect_simple(TokenKind::As, "Expected 'As' in Name statement")?;
        let new_path = self.parse_expression()?;
        let end = new_path.span;
        Ok(Stmt::NameFile {
            old_path,
            new_path,
            span: Span::new(self.file_id, start_span.start, end.end),
        })
    }

    fn parse_file_number_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.expect_simple(TokenKind::Hash, "Expected '#' before file number")?;
        self.parse_expression()
    }

    fn parse_open_file_number_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.match_simple(&TokenKind::Hash);
        self.parse_expression()
    }

    fn parse_let_assignment(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Let, "Expected 'Let'")?.span;
        let target = self.parse_assignment_target()?;
        self.expect_simple(TokenKind::Equal, "Expected '=' in Let assignment")?;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Assign {
            target,
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_set_assignment(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Set, "Expected 'Set'")?.span;
        let target = self.parse_assignment_target()?;
        self.expect_simple(TokenKind::Equal, "Expected '=' in Set assignment")?;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::SetAssign {
            target,
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_identifier_statement_with_expr(
        &mut self,
        expr: Expr,
        start: Span,
    ) -> Result<Stmt, Diagnostic> {
        let span = expr.span;
        match &expr.kind {
            ExprKind::Call { name, args, .. } => {
                if self.match_simple(&TokenKind::Equal) {
                    let value = self.parse_expression()?;
                    let end = value.span;
                    return Ok(Stmt::Assign {
                        target: AssignTarget::ArrayElement {
                            name: name.clone(),
                            indices: args.clone(),
                            span,
                        },
                        expr: value,
                        span: Span::new(self.file_id, start.start, end.end),
                    });
                }

                // Check if it's a member access after the call (e.g., obj(1).prop = 2)
                if self.check_simple(&TokenKind::Dot) {
                    let target = self.parse_member_access(expr)?;
                    let target_span = target.span;
                    let ExprKind::MemberAccess { object, field } = target.kind else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "Expected member assignment target",
                            Some(target_span),
                        ));
                    };
                    self.expect_simple(TokenKind::Equal, "Expected '=' in member assignment")?;
                    let value = self.parse_expression()?;
                    let end = value.span;
                    return Ok(Stmt::Assign {
                        target: AssignTarget::Member {
                            object: *object,
                            field,
                            span: target_span,
                        },
                        expr: value,
                        span: Span::new(self.file_id, target_span.start, end.end),
                    });
                }

                Ok(Stmt::SubCall {
                    name: name.clone(),
                    args: args.clone(),
                    span: Span::new(self.file_id, start.start, span.end),
                })
            }
            ExprKind::MemberAccess { object, field } => {
                if let ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Debug")
                    && field.eq_ignore_ascii_case("Print")
                {
                    let args = self.parse_bare_call_arguments()?;
                    let end = args.last().map(|arg| arg.span).unwrap_or(span);
                    return Ok(Stmt::DebugPrint {
                        args,
                        span: Span::new(self.file_id, start.start, end.end),
                    });
                }

                if self.match_simple(&TokenKind::Equal) {
                    let value = self.parse_expression()?;
                    let end = value.span;
                    return Ok(Stmt::Assign {
                        target: AssignTarget::Member {
                            object: (**object).clone(),
                            field: field.clone(),
                            span,
                        },
                        expr: value,
                        span: Span::new(self.file_id, start.start, end.end),
                    });
                }

                // Bare member call
                let args = self.parse_bare_call_arguments()?;
                let end = args.last().map(|arg| arg.span).unwrap_or(span);
                Ok(Stmt::MemberSubCall {
                    object: (**object).clone(),
                    method: field.clone(),
                    args,
                    span: Span::new(self.file_id, start.start, end.end),
                })
            }
            ExprKind::MemberCall {
                object,
                method,
                type_args: _,
                args,
            } => {
                if let ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Debug")
                    && method.eq_ignore_ascii_case("Print")
                {
                    return Ok(Stmt::DebugPrint {
                        args: args.clone(),
                        span,
                    });
                }

                if self.match_simple(&TokenKind::Equal) {
                    let value = self.parse_expression()?;
                    let end = value.span;
                    return Ok(Stmt::Assign {
                        target: AssignTarget::MemberArrayElement {
                            object: (**object).clone(),
                            field: method.clone(),
                            indices: args.clone(),
                            span,
                        },
                        expr: value,
                        span: Span::new(self.file_id, start.start, end.end),
                    });
                }

                Ok(Stmt::MemberSubCall {
                    object: (**object).clone(),
                    method: method.clone(),
                    args: args.clone(),
                    span: Span::new(self.file_id, start.start, span.end),
                })
            }
            ExprKind::Variable(name) => {
                if self.match_simple(&TokenKind::Equal) {
                    let value = self.parse_expression()?;
                    let end = value.span;
                    return Ok(Stmt::Assign {
                        target: AssignTarget::Variable {
                            name: name.clone(),
                            span,
                        },
                        expr: value,
                        span: Span::new(self.file_id, start.start, end.end),
                    });
                }

                let args = self.parse_bare_call_arguments()?;
                let end = args.last().map(|arg| arg.span).unwrap_or(span);
                Ok(Stmt::SubCall {
                    name: name.clone(),
                    args,
                    span: Span::new(self.file_id, start.start, end.end),
                })
            }
            _ => {
                if self.match_simple(&TokenKind::Equal) {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::PARSE,
                        "Invalid assignment target",
                        Some(span),
                    ));
                }

                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected statement",
                    Some(span),
                ))
            }
        }
    }

    fn parse_identifier_statement(&mut self) -> Result<Stmt, Diagnostic> {
        if matches!(self.peek_kind(), TokenKind::Identifier(_, _))
            && matches!(self.peek_next_kind(), Some(TokenKind::Colon))
        {
            let token = self.advance();
            let TokenKind::Identifier(name, _) = token.kind else {
                unreachable!("peek checked");
            };
            let colon = self.expect_simple(TokenKind::Colon, "Expected ':' after label")?;
            return Ok(Stmt::Label {
                name,
                span: Span::new(self.file_id, token.span.start, colon.span.end),
            });
        }

        let start_span = self.peek().span;
        let expr = self.parse_primary()?;
        self.parse_identifier_statement_with_expr(expr, start_span)
    }

    fn parse_call_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Call, "Expected 'Call'")?.span;
        let target = self.parse_call_statement_target()?;
        match target.kind {
            ExprKind::Call { name, args, .. } => Ok(Stmt::SubCall {
                name,
                args,
                span: Span::new(self.file_id, start.start, target.span.end),
            }),
            ExprKind::MemberCall {
                object,
                method,
                type_args: _,
                args,
            } => Ok(Stmt::MemberSubCall {
                object: *object,
                method,
                args,
                span: Span::new(self.file_id, start.start, target.span.end),
            }),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Call statement requires a Sub call",
                Some(target.span),
            )),
        }
    }

    fn parse_raise_event(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::RaiseEvent, "Expected 'RaiseEvent'")?
            .span;
        let name = self.expect_identifier("Expected event name after 'RaiseEvent'")?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after event name")?;
        let args = self.finish_call_arguments()?;
        let end = self.previous().span;
        Ok(Stmt::RaiseEvent {
            name,
            args,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_call_statement_target(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.peek().span;
        let base = match self.advance().kind {
            TokenKind::Identifier(name, _) => {
                if self.check_simple(&TokenKind::Dot) {
                    Expr {
                        kind: ExprKind::Variable(name),
                        span: start,
                    }
                } else {
                    let args = if self.match_simple(&TokenKind::LeftParen) {
                        self.finish_call_arguments()?
                    } else {
                        Vec::new()
                    };
                    Expr {
                        kind: ExprKind::Call {
                            name,
                            type_args: Vec::new(),
                            args,
                        },
                        span: Span::new(self.file_id, start.start, self.previous().span.end),
                    }
                }
            }
            TokenKind::Console => Expr {
                kind: ExprKind::Variable("Console".to_string()),
                span: start,
            },
            TokenKind::Me => Expr {
                kind: ExprKind::Me,
                span: start,
            },
            TokenKind::Dot => {
                self.current -= 1;
                return self.parse_primary();
            }
            other => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    format!("Expected call target after 'Call', found {:?}", other),
                    Some(start),
                ));
            }
        };
        self.parse_member_access(base)
    }

    fn parse_bare_call_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        let mut saw_named = false;
        while !self.at_statement_separator() {
            let arg = self.parse_argument()?;
            if matches!(arg.kind, ExprKind::NamedArg { .. }) {
                saw_named = true;
            } else if saw_named {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Positional arguments cannot appear after named arguments",
                    Some(arg.span),
                ));
            }
            args.push(arg);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn at_statement_separator(&self) -> bool {
        self.check_simple(&TokenKind::Newline)
            || self.check_simple(&TokenKind::Colon)
            || self.check_simple(&TokenKind::Eof)
            || self.matches_any_block_boundary()
    }

    fn parse_assignment_target(&mut self) -> Result<AssignTarget, Diagnostic> {
        let expr = self.parse_primary()?;
        let span = expr.span;
        match expr.kind {
            ExprKind::Variable(name) => Ok(AssignTarget::Variable { name, span }),
            ExprKind::Call { name, args, .. } => Ok(AssignTarget::ArrayElement {
                name,
                indices: args,
                span,
            }),
            ExprKind::MemberAccess { object, field } => Ok(AssignTarget::Member {
                object: *object,
                field,
                span,
            }),
            ExprKind::MemberCall {
                object,
                method,
                type_args: _,
                args,
            } => Ok(AssignTarget::MemberArrayElement {
                object: *object,
                field: method,
                indices: args,
                span,
            }),
            ExprKind::PassingModeOverride { .. } => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                "Passing mode override is not allowed in assignment targets",
                Some(span),
            )),
            ExprKind::Me => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT,
                "Me is not assignable",
                Some(span),
            )),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Expected assignment target",
                Some(span),
            )),
        }
    }

    fn parse_console_writeline(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Console, "Expected 'Console'")?
            .span;
        self.expect_simple(TokenKind::Dot, "Expected '.' after 'Console'")?;

        let method = match self.advance().kind {
            TokenKind::WriteLine => "WriteLine".to_string(),
            TokenKind::Identifier(name, _) => name,
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected method name after 'Console.'",
                    Some(self.previous().span),
                ));
            }
        };

        let args = if self.match_simple(&TokenKind::LeftParen) {
            let mut args = Vec::new();
            if !self.check_simple(&TokenKind::RightParen) {
                loop {
                    args.push(self.parse_expression()?);
                    if !self.match_simple(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect_simple(TokenKind::RightParen, "Expected ')' after arguments")?;
            args
        } else {
            self.parse_bare_call_arguments()?
        };

        let end = self.previous().span;
        Ok(Stmt::ConsoleCall {
            method,
            args,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_end_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.advance().span;
        if !self.at_statement_separator() && !self.is_at_end() {
            return Err(self.error_here("Expected statement"));
        }
        Ok(Stmt::End { span: start })
    }

    fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Return, "Expected 'Return'")?
            .span;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Return {
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_yield(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Yield, "Expected 'Yield'")?
            .span;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Yield {
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_throw(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Throw, "Expected 'Throw'")?
            .span;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Throw {
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::If, "Expected 'If'")?.span;
        let condition = self.parse_expression()?;
        self.expect_simple(TokenKind::Then, "Expected 'Then' after If condition")?;

        if !self.check_simple(&TokenKind::Newline) && !self.is_at_end() {
            // Single-line If
            let mut then_body = Vec::new();
            while !self.check_simple(&TokenKind::Else) && !self.at_statement_separator() {
                then_body.push(self.parse_stmt()?);
                if !self.match_simple(&TokenKind::Colon) {
                    break;
                }
            }

            let else_body = if self.match_simple(&TokenKind::Else) {
                let mut body = Vec::new();
                while !self.at_statement_separator() {
                    body.push(self.parse_stmt()?);
                    if !self.match_simple(&TokenKind::Colon) {
                        break;
                    }
                }
                body
            } else {
                Vec::new()
            };

            let end = self.previous().span;
            return Ok(Stmt::If {
                condition,
                then_body,
                elseif_branches: Vec::new(),
                else_body,
                span: Span::new(self.file_id, start.start, end.end),
            });
        }

        self.expect_newline("Expected newline after 'Then' or single-line statement")?;

        let then_body =
            self.parse_block_until(&[BlockEnd::ElseIf, BlockEnd::Else, BlockEnd::EndIf])?;
        let mut elseif_branches = Vec::new();
        while self.match_simple(&TokenKind::ElseIf) {
            let condition = self.parse_expression()?;
            self.expect_simple(TokenKind::Then, "Expected 'Then' after ElseIf condition")?;
            self.expect_newline("Expected newline after 'Then'")?;
            let body =
                self.parse_block_until(&[BlockEnd::ElseIf, BlockEnd::Else, BlockEnd::EndIf])?;
            elseif_branches.push(ElseIfBranch { condition, body });
        }
        let else_body = if self.match_simple(&TokenKind::Else) {
            self.expect_newline("Expected newline after 'Else'")?;
            self.parse_block_until(&[BlockEnd::EndIf])?
        } else {
            Vec::new()
        };

        if !self.matches_block_end(&[BlockEnd::EndIf]) {
            return Err(self.error_here("Expected 'End If'"));
        }
        self.expect_simple(TokenKind::End, "Expected 'End If'")?;
        let end = self
            .expect_simple(TokenKind::If, "Expected 'If' after 'End'")?
            .span;

        Ok(Stmt::If {
            condition,
            then_body,
            elseif_branches,
            else_body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::While, "Expected 'While'")?
            .span;
        let condition = self.parse_expression()?;
        self.expect_newline("Expected newline after While condition")?;
        let body = self.parse_block_until(&[BlockEnd::Wend])?;
        let end = self.expect_simple(TokenKind::Wend, "Expected 'Wend'")?.span;

        Ok(Stmt::While {
            condition,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_with(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::With, "Expected 'With'")?.span;
        let target = self.parse_expression()?;
        self.expect_newline("Expected newline after With expression")?;
        let body = self.parse_block_until(&[BlockEnd::EndWith])?;
        self.expect_simple(TokenKind::End, "Expected 'End With'")?;
        let end = self
            .expect_simple(TokenKind::With, "Expected 'With' after 'End'")?
            .span;

        Ok(Stmt::With {
            target,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_using(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Using, "Expected 'Using'")?
            .span;
        let resource = if matches!(self.peek_kind(), TokenKind::Identifier(_, _))
            && matches!(self.peek_next_kind(), Some(TokenKind::As))
        {
            UsingResource::Declaration(self.parse_variable_declarator("Using")?)
        } else {
            UsingResource::Target(self.parse_expression()?)
        };
        self.expect_newline("Expected newline after Using resource")?;
        let body = self.parse_block_until(&[BlockEnd::EndUsing])?;
        self.expect_simple(TokenKind::End, "Expected 'End Using'")?;
        let end = self
            .expect_simple(TokenKind::Using, "Expected 'Using' after 'End'")?
            .span;

        Ok(Stmt::Using {
            resource,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_select_case(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Select, "Expected 'Select'")?
            .span;
        self.expect_simple(TokenKind::Case, "Expected 'Case' after 'Select'")?;
        let subject = self.parse_expression()?;
        self.expect_newline("Expected newline after Select Case expression")?;

        let mut branches = Vec::new();
        let mut else_body = Vec::new();
        let mut saw_else = false;
        self.skip_newlines();
        while !self.is_at_end() && !self.matches_block_end(&[BlockEnd::EndSelect]) {
            if !self.match_simple(&TokenKind::Case) {
                return Err(self.error_here("Expected 'Case' or 'End Select'"));
            }

            if self.match_simple(&TokenKind::Else) {
                if saw_else {
                    return Err(self.error_here("Case Else is already declared"));
                }
                saw_else = true;
                else_body = self.parse_case_body("Expected newline after Case Else")?;
                if self.matches_block_end(&[BlockEnd::Case]) {
                    return Err(self.error_here("Case Else must be last"));
                }
            } else {
                if saw_else {
                    return Err(self.error_here("Case Else must be last"));
                }
                let items = self.parse_case_items()?;
                let body = self.parse_case_body("Expected newline after Case values")?;
                branches.push(CaseBranch { items, body });
            }
            self.skip_newlines();
        }

        if !self.matches_block_end(&[BlockEnd::EndSelect]) {
            return Err(self.error_here("Expected 'End Select'"));
        }
        self.expect_simple(TokenKind::End, "Expected 'End Select'")?;
        let end = self
            .expect_simple(TokenKind::Select, "Expected 'Select' after 'End'")?
            .span;

        Ok(Stmt::SelectCase {
            subject,
            branches,
            else_body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_case_items(&mut self) -> Result<Vec<CaseItem>, Diagnostic> {
        let mut items = Vec::new();
        loop {
            items.push(self.parse_case_item()?);
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        Ok(items)
    }

    fn parse_case_item(&mut self) -> Result<CaseItem, Diagnostic> {
        if self.match_simple(&TokenKind::Is) {
            let op = self.parse_case_compare_op()?;
            let expr = self.parse_expression()?;
            return Ok(CaseItem::Compare { op, expr });
        }

        let start = self.parse_expression()?;
        if self.match_simple(&TokenKind::To) {
            let end = self.parse_expression()?;
            Ok(CaseItem::Range { start, end })
        } else {
            Ok(CaseItem::Value(start))
        }
    }

    fn parse_case_compare_op(&mut self) -> Result<CaseCompareOp, Diagnostic> {
        let token = self.advance();
        match token.kind {
            TokenKind::Equal => Ok(CaseCompareOp::Equal),
            TokenKind::NotEqual => Ok(CaseCompareOp::NotEqual),
            TokenKind::Less => Ok(CaseCompareOp::Less),
            TokenKind::Greater => Ok(CaseCompareOp::Greater),
            TokenKind::LessEqual => Ok(CaseCompareOp::LessEqual),
            TokenKind::GreaterEqual => Ok(CaseCompareOp::GreaterEqual),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Expected comparison operator after 'Case Is'",
                Some(token.span),
            )),
        }
    }

    fn parse_case_body(&mut self, newline_message: &str) -> Result<Vec<Stmt>, Diagnostic> {
        if self.match_simple(&TokenKind::Colon) {
            let stmt = self.parse_stmt()?;
            Ok(vec![stmt])
        } else {
            self.expect_newline(newline_message)?;
            self.parse_block_until(&[BlockEnd::Case, BlockEnd::EndSelect])
        }
    }

    fn parse_do_loop(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Do, "Expected 'Do'")?.span;
        let pre_condition = if self.match_simple(&TokenKind::While) {
            Some((true, self.parse_expression()?))
        } else if self.match_simple(&TokenKind::Until) {
            Some((false, self.parse_expression()?))
        } else {
            None
        };

        if self.match_simple(&TokenKind::Colon) {
            let mut body = Vec::new();
            while !self.check_simple(&TokenKind::Loop) && !self.is_at_end() {
                body.push(self.parse_stmt()?);
                if !self.match_simple(&TokenKind::Colon) {
                    break;
                }
            }
            self.expect_simple(TokenKind::Loop, "Expected 'Loop' in inline Do loop")?;
            let loop_span = self.previous().span;

            let condition = if let Some((is_while, condition)) = pre_condition {
                if is_while {
                    DoLoopCondition::PreWhile(condition)
                } else {
                    DoLoopCondition::PreUntil(condition)
                }
            } else if self.match_simple(&TokenKind::While) {
                DoLoopCondition::PostWhile(self.parse_expression()?)
            } else if self.match_simple(&TokenKind::Until) {
                DoLoopCondition::PostUntil(self.parse_expression()?)
            } else {
                DoLoopCondition::Infinite
            };

            let end = match &condition {
                DoLoopCondition::PostWhile(expr) | DoLoopCondition::PostUntil(expr) => expr.span,
                _ => loop_span,
            };

            return Ok(Stmt::DoLoop {
                condition,
                body,
                span: Span::new(self.file_id, start.start, end.end),
            });
        }

        self.expect_newline("Expected newline after Do statement")?;

        let body = self.parse_block_until(&[BlockEnd::Loop])?;
        if !self.matches_block_end(&[BlockEnd::Loop]) {
            return Err(self.error_here("Expected 'Loop'"));
        }
        let loop_token = self.expect_simple(TokenKind::Loop, "Expected 'Loop'")?;

        let condition = if let Some((is_while, condition)) = pre_condition {
            if self.check_simple(&TokenKind::While) || self.check_simple(&TokenKind::Until) {
                return Err(
                    self.error_here("Do loop cannot have both pre-test and post-test conditions")
                );
            }
            if is_while {
                DoLoopCondition::PreWhile(condition)
            } else {
                DoLoopCondition::PreUntil(condition)
            }
        } else if self.match_simple(&TokenKind::While) {
            DoLoopCondition::PostWhile(self.parse_expression()?)
        } else if self.match_simple(&TokenKind::Until) {
            DoLoopCondition::PostUntil(self.parse_expression()?)
        } else {
            DoLoopCondition::Infinite
        };
        let end = match &condition {
            DoLoopCondition::PostWhile(expr) | DoLoopCondition::PostUntil(expr) => expr.span,
            _ => loop_token.span,
        };

        Ok(Stmt::DoLoop {
            condition,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::For, "Expected 'For'")?.span;
        if self.match_simple(&TokenKind::Each) {
            return self.parse_for_each(start);
        }
        let variable = self.expect_identifier("Expected loop variable after 'For'")?;
        self.expect_simple(TokenKind::Equal, "Expected '=' after loop variable")?;
        let start_expr = self.parse_expression()?;
        self.expect_simple(TokenKind::To, "Expected 'To' in For statement")?;
        let end_expr = self.parse_expression()?;
        let step = if self.match_simple(&TokenKind::Step) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect_newline("Expected newline after For statement")?;

        let body = self.parse_block_until(&[BlockEnd::Next])?;
        let next = self.expect_simple(TokenKind::Next, "Expected 'Next'")?;
        let next_variable = match self.peek_kind() {
            TokenKind::Identifier(_, _) => {
                let token = self.advance();
                let TokenKind::Identifier(name, _) = token.kind else {
                    unreachable!("peek checked");
                };
                Some((name, token.span))
            }
            _ => None,
        };
        let end = next_variable
            .as_ref()
            .map(|(_, span)| *span)
            .unwrap_or(next.span);

        Ok(Stmt::For {
            variable,
            start: start_expr,
            end: end_expr,
            step,
            next_variable,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_for_each(&mut self, start: Span) -> Result<Stmt, Diagnostic> {
        let variable = self.expect_identifier("Expected loop variable after 'For Each'")?;
        self.expect_simple(TokenKind::In, "Expected 'In' in For Each statement")?;
        let iterable = self.parse_expression()?;
        self.expect_newline("Expected newline after For Each statement")?;

        let body = self.parse_block_until(&[BlockEnd::Next])?;
        let next = self.expect_simple(TokenKind::Next, "Expected 'Next'")?;
        let next_variable = match self.peek_kind() {
            TokenKind::Identifier(_, _) => {
                let token = self.advance();
                let TokenKind::Identifier(name, _) = token.kind else {
                    unreachable!("peek checked");
                };
                Some((name, token.span))
            }
            _ => None,
        };
        let end = next_variable
            .as_ref()
            .map(|(_, span)| *span)
            .unwrap_or(next.span);

        Ok(Stmt::ForEach {
            variable,
            iterable,
            next_variable,
            body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_redim(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::ReDim, "Expected 'ReDim'")?
            .span;
        let preserve = self.match_simple(&TokenKind::Preserve);
        let target = self.parse_redim_target("ReDim")?;

        self.expect_simple(TokenKind::LeftParen, "Expected '(' after array name")?;
        let mut dims = Vec::new();
        loop {
            let upper_or_lower = self.parse_expression()?;
            if self.match_simple(&TokenKind::To) {
                let upper = self.parse_expression()?;
                dims.push((Some(upper_or_lower), upper));
            } else {
                dims.push((None, upper_or_lower));
            }
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "Expected ')' after array dimensions")?;
        let end = self.previous().span;
        Ok(Stmt::ReDim {
            target,
            dims,
            preserve,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_erase(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Erase, "Expected 'Erase'")?
            .span;
        let target = self.parse_redim_target("Erase")?;
        let end = self.previous().span;
        Ok(Stmt::Erase {
            target,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_lset(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::LSet, "Expected 'LSet'")?.span;
        let target = self.parse_assignment_target()?;
        self.expect_simple(TokenKind::Equal, "Expected '=' after LSet target")?;
        let expr = self.parse_expression()?;
        let end = self.previous().span;
        Ok(Stmt::LSet {
            target,
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_rset(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::RSet, "Expected 'RSet'")?.span;
        let target = self.parse_assignment_target()?;
        self.expect_simple(TokenKind::Equal, "Expected '=' after RSet target")?;
        let expr = self.parse_expression()?;
        let end = self.previous().span;
        Ok(Stmt::RSet {
            target,
            expr,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_redim_target(&mut self, context: &str) -> Result<ReDimTarget, Diagnostic> {
        let start = self.peek().span;
        if self.match_simple(&TokenKind::Me) {
            let object = Expr {
                kind: ExprKind::Me,
                span: start,
            };
            self.expect_simple(
                TokenKind::Dot,
                &format!("Expected '.' after 'Me' in {} target", context),
            )?;
            let field =
                self.expect_identifier(&format!("Expected field name after 'Me.' in {}", context))?;
            Ok(ReDimTarget::Member {
                object,
                field,
                span: Span::new(self.file_id, start.start, self.previous().span.end),
            })
        } else {
            let name =
                self.expect_identifier(&format!("Expected array name after '{}'", context))?;
            if self.match_simple(&TokenKind::Dot) {
                let object = Expr {
                    kind: ExprKind::Variable(name),
                    span: start,
                };
                let field = self
                    .expect_identifier(&format!("Expected field name after '.' in {}", context))?;
                Ok(ReDimTarget::Member {
                    object,
                    field,
                    span: Span::new(self.file_id, start.start, self.previous().span.end),
                })
            } else {
                Ok(ReDimTarget::Variable { name, span: start })
            }
        }
    }

    fn parse_goto(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::GoTo, "Expected 'GoTo'")?.span;
        let token = self.advance();
        let label = match token.kind {
            TokenKind::Identifier(label, _) => label,
            TokenKind::Integer(number) => number.to_string(),
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected label name after 'GoTo'",
                    Some(token.span),
                ));
            }
        };
        Ok(Stmt::GoTo {
            label,
            span: Span::new(self.file_id, start.start, token.span.end),
        })
    }

    fn parse_on_error(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::On, "Expected 'On'")?.span;
        self.expect_simple(TokenKind::Error, "Expected 'Error' after 'On'")?;
        let mode = if self.match_simple(&TokenKind::Resume) {
            self.expect_simple(TokenKind::Next, "Expected 'Next' after 'On Error Resume'")?;
            OnErrorMode::ResumeNext
        } else if self.match_simple(&TokenKind::GoTo) {
            let token = self.advance();
            match token.kind {
                TokenKind::Integer(0) => OnErrorMode::GoToZero,
                TokenKind::Minus => {
                    let one = self.advance();
                    if matches!(one.kind, TokenKind::Integer(1)) {
                        OnErrorMode::GoToMinusOne
                    } else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "On Error GoTo requires 0, -1, or a label",
                            Some(one.span),
                        ));
                    }
                }
                TokenKind::Integer(number) => OnErrorMode::GoToLabel(number.to_string()),
                TokenKind::Identifier(label, _) => OnErrorMode::GoToLabel(label),
                _ => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "On Error GoTo requires 0, -1, or a label",
                        Some(token.span),
                    ));
                }
            }
        } else {
            return Err(self.error_here(
                "Expected 'Resume Next', 'GoTo 0', 'GoTo -1', or 'GoTo <label>' after 'On Error'",
            ));
        };
        let end = self.previous().span;
        Ok(Stmt::OnError {
            mode,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_resume(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Resume, "Expected 'Resume'")?
            .span;
        let target = match self.peek_kind() {
            TokenKind::Next => {
                let next = self.advance();
                return Ok(Stmt::Resume {
                    target: ResumeTarget::Next,
                    span: Span::new(self.file_id, start.start, next.span.end),
                });
            }
            TokenKind::Identifier(_, _) => {
                let token = self.advance();
                let TokenKind::Identifier(label, _) = token.kind else {
                    unreachable!("peek checked");
                };
                return Ok(Stmt::Resume {
                    target: ResumeTarget::Label(label),
                    span: Span::new(self.file_id, start.start, token.span.end),
                });
            }
            TokenKind::Integer(_) => {
                let token = self.advance();
                let TokenKind::Integer(number) = token.kind else {
                    unreachable!("peek checked");
                };
                return Ok(Stmt::Resume {
                    target: ResumeTarget::Label(number.to_string()),
                    span: Span::new(self.file_id, start.start, token.span.end),
                });
            }
            _ => ResumeTarget::Retry,
        };
        Ok(Stmt::Resume {
            target,
            span: start,
        })
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Try, "Expected 'Try'")?.span;
        self.expect_newline("Expected newline after 'Try'")?;

        let try_body =
            self.parse_block_until(&[BlockEnd::Catch, BlockEnd::Finally, BlockEnd::EndTry])?;

        let catch_block = if self.match_simple(&TokenKind::Catch) {
            let catch_start = self.previous().span;
            let variable = if matches!(self.peek_kind(), TokenKind::Identifier(_, _)) {
                let name = self.expect_identifier("Expected catch variable name")?;
                self.expect_simple(TokenKind::As, "Expected 'As' after catch variable")?;
                let ty_token = self.advance();
                match ty_token.kind {
                    TokenKind::Error => {}
                    TokenKind::Identifier(ref ty_name, _)
                        if ty_name.eq_ignore_ascii_case("Error") => {}
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "Catch variable must be of type 'Error'",
                            Some(ty_token.span),
                        ));
                    }
                }
                Some(name)
            } else {
                None
            };
            self.expect_newline("Expected newline after 'Catch'")?;
            let body = self.parse_block_until(&[BlockEnd::Finally, BlockEnd::EndTry])?;
            let catch_end = self.previous().span;
            Some(CatchBlock {
                variable,
                body,
                span: Span::new(self.file_id, catch_start.start, catch_end.end),
            })
        } else {
            None
        };

        let finally_body = if self.match_simple(&TokenKind::Finally) {
            self.expect_newline("Expected newline after 'Finally'")?;
            Some(self.parse_block_until(&[BlockEnd::EndTry])?)
        } else {
            None
        };

        if catch_block.is_none() && finally_body.is_none() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Try statement must have at least one Catch or Finally block",
                Some(start),
            ));
        }

        self.expect_simple(TokenKind::End, "Expected 'End Try'")?;
        let end = self
            .expect_simple(TokenKind::Try, "Expected 'Try' after 'End'")?
            .span;

        Ok(Stmt::TryCatch {
            try_body,
            catch_block,
            finally_body,
            span: Span::new(self.file_id, start.start, end.end),
        })
    }

    fn parse_exit(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Exit, "Expected 'Exit'")?.span;
        let token = self.advance();
        let target = match token.kind {
            TokenKind::Sub => ExitTarget::Sub,
            TokenKind::Function => ExitTarget::Function,
            TokenKind::For => ExitTarget::For,
            TokenKind::While => ExitTarget::While,
            TokenKind::Do => ExitTarget::Do,
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected 'Sub', 'Function', 'For', 'While', or 'Do' after 'Exit'",
                    Some(token.span),
                ));
            }
        };
        Ok(Stmt::Exit {
            target,
            span: Span::new(self.file_id, start.start, token.span.end),
        })
    }

    pub(super) fn matches_block_end(&self, ends: &[BlockEnd]) -> bool {
        ends.iter().any(|end| match end {
            BlockEnd::Else => matches!(self.peek_kind(), TokenKind::Else),
            BlockEnd::ElseIf => matches!(self.peek_kind(), TokenKind::ElseIf),
            BlockEnd::Wend => matches!(self.peek_kind(), TokenKind::Wend),
            BlockEnd::Next => matches!(self.peek_kind(), TokenKind::Next),
            BlockEnd::Loop => matches!(self.peek_kind(), TokenKind::Loop),
            BlockEnd::Case => matches!(self.peek_kind(), TokenKind::Case),
            BlockEnd::EndIf => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::If))
            }
            BlockEnd::EndSub => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Sub))
            }
            BlockEnd::EndGet => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Get))
            }
            BlockEnd::EndSet => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Set))
            }
            BlockEnd::EndFunction => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Function))
            }
            BlockEnd::EndProperty => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Property))
            }
            BlockEnd::EndSelect => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Select))
            }
            BlockEnd::EndType => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Type))
            }
            BlockEnd::EndStructure => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Structure))
            }
            BlockEnd::EndInterface => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Interface))
            }
            BlockEnd::EndClass => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Class))
            }
            BlockEnd::EndEnum => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Enum))
            }
            BlockEnd::EndWith => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::With))
            }
            BlockEnd::EndUsing => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Using))
            }
            BlockEnd::Catch => matches!(self.peek_kind(), TokenKind::Catch),
            BlockEnd::Finally => matches!(self.peek_kind(), TokenKind::Finally),
            BlockEnd::EndTry => {
                matches!(self.peek_kind(), TokenKind::End)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Try))
            }
        })
    }

    pub(super) fn matches_any_block_boundary(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Else
                | TokenKind::ElseIf
                | TokenKind::Wend
                | TokenKind::Next
                | TokenKind::Loop
                | TokenKind::Case
                | TokenKind::Catch
                | TokenKind::Finally
        ) || (matches!(self.peek_kind(), TokenKind::End)
            && matches!(
                self.peek_next_kind(),
                Some(
                    TokenKind::If
                        | TokenKind::Sub
                        | TokenKind::Get
                        | TokenKind::Set
                        | TokenKind::Function
                        | TokenKind::Property
                        | TokenKind::Select
                        | TokenKind::Type
                        | TokenKind::Structure
                        | TokenKind::Enum
                        | TokenKind::Class
                        | TokenKind::With
                        | TokenKind::Using
                        | TokenKind::Try
                )
            ))
    }
}
