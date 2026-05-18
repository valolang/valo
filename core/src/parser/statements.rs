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
            self.expect_statement_end("Expected newline after statement")?;
            self.skip_newlines();
        }

        Ok(statements)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        match self.peek_kind() {
            TokenKind::Dim => self.parse_dim(),
            TokenKind::Const => self.parse_const_stmt(),
            TokenKind::If => self.parse_if(),
            TokenKind::Select => self.parse_select_case(),
            TokenKind::While => self.parse_while(),
            TokenKind::With => self.parse_with(),
            TokenKind::Do => self.parse_do_loop(),
            TokenKind::For => self.parse_for(),
            TokenKind::GoTo => self.parse_goto(),
            TokenKind::On => self.parse_on_error(),
            TokenKind::Resume => self.parse_resume(),
            TokenKind::Exit => self.parse_exit(),
            TokenKind::ReDim => self.parse_redim(),
            TokenKind::Let => self.parse_let_assignment(),
            TokenKind::Call => self.parse_call_statement(),
            TokenKind::Set => self.parse_set_assignment(),
            TokenKind::Console => self.parse_console_writeline(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Identifier(_) | TokenKind::Me | TokenKind::Dot => {
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
        let name = self.expect_identifier("Expected variable name after 'Dim'")?;
        let array = if self.match_simple(&TokenKind::LeftParen) {
            if self.match_simple(&TokenKind::RightParen) {
                Some(ArrayDecl::Dynamic)
            } else {
                let size_token = self.advance();
                let TokenKind::Integer(size) = size_token.kind else {
                    return Err(Diagnostic::new(
                        "Array size must be an Integer literal",
                        Some(size_token.span),
                    ));
                };
                if size < 0 {
                    return Err(Diagnostic::new(
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
        self.expect_simple(TokenKind::As, "Expected 'As' in variable declaration")?;
        let ty = self.parse_type_name()?;
        let end = self.previous().span;

        Ok(Stmt::Dim {
            name,
            ty,
            array,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_const_stmt(&mut self) -> Result<Stmt, Diagnostic> {
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
        Ok(Stmt::Const {
            name,
            ty,
            value,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_assignment(&mut self) -> Result<Stmt, Diagnostic> {
        let target = self.parse_assignment_target()?;
        let start = target.span();
        self.expect_simple(TokenKind::Equal, "Expected '=' in assignment")?;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Assign {
            target,
            expr,
            span: Span::new(start.start, end.end),
        })
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
            span: Span::new(start.start, end.end),
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
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_identifier_statement(&mut self) -> Result<Stmt, Diagnostic> {
        if matches!(self.peek_kind(), TokenKind::Identifier(_))
            && matches!(self.peek_next_kind(), Some(TokenKind::Colon))
        {
            let token = self.advance();
            let TokenKind::Identifier(name) = token.kind else {
                unreachable!("peek checked");
            };
            let colon = self.expect_simple(TokenKind::Colon, "Expected ':' after label")?;
            return Ok(Stmt::Label {
                name,
                span: Span::new(token.span.start, colon.span.end),
            });
        }
        if matches!(self.peek_kind(), TokenKind::Dot) {
            return self.parse_member_assignment();
        }
        match self.peek_next_kind() {
            Some(TokenKind::LeftParen) => self.parse_call_or_array_assignment(),
            Some(TokenKind::Dot) => self.parse_member_assignment(),
            _ if matches!(self.peek_kind(), TokenKind::Me) => self.parse_member_assignment(),
            _ => self.parse_assignment(),
        }
    }

    fn parse_call_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::Call, "Expected 'Call'")?.span;
        let target = self.parse_primary()?;
        match target.kind {
            ExprKind::Call { name, args } => Ok(Stmt::SubCall {
                name,
                args,
                span: Span::new(start.start, target.span.end),
            }),
            ExprKind::MemberCall {
                object,
                method,
                args,
            } => Ok(Stmt::MemberSubCall {
                object: *object,
                method,
                args,
                span: Span::new(start.start, target.span.end),
            }),
            _ => Err(Diagnostic::new(
                "Call statement requires a Sub call",
                Some(target.span),
            )),
        }
    }

    fn parse_call_or_array_assignment(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.peek().span;
        let name = self.expect_identifier("Expected name")?;
        let args = self.parse_call_arguments()?;
        let target_end = self.previous().span;

        if self.match_simple(&TokenKind::Equal) {
            if args.len() != 1 {
                return Err(Diagnostic::new(
                    "Array assignment requires exactly one index",
                    Some(start),
                ));
            }
            let mut args = args.into_iter();
            let index = args.next().expect("len checked");
            let expr = self.parse_expression()?;
            let end = expr.span;
            return Ok(Stmt::Assign {
                target: AssignTarget::ArrayElement {
                    name,
                    index,
                    span: Span::new(start.start, target_end.end),
                },
                expr,
                span: Span::new(start.start, end.end),
            });
        }

        if self.check_simple(&TokenKind::Dot) {
            let target = self.parse_member_access(Expr {
                kind: ExprKind::Call { name, args },
                span: Span::new(start.start, self.previous().span.end),
            })?;
            let target_span = target.span;
            let ExprKind::MemberAccess { object, field } = target.kind else {
                return Err(Diagnostic::new(
                    "Expected member assignment target",
                    Some(target_span),
                ));
            };
            self.expect_simple(TokenKind::Equal, "Expected '=' in member assignment")?;
            let expr = self.parse_expression()?;
            let end = expr.span;
            return Ok(Stmt::Assign {
                target: AssignTarget::Member {
                    object: *object,
                    field,
                    span: target_span,
                },
                expr,
                span: Span::new(target_span.start, end.end),
            });
        }

        let end = self.previous().span;
        Ok(Stmt::SubCall {
            name,
            args,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_member_assignment(&mut self) -> Result<Stmt, Diagnostic> {
        let target = self.parse_primary()?;
        let target_span = target.span;
        let (object, field, args) = match target.kind {
            ExprKind::MemberAccess { object, field } => (object, field, None),
            ExprKind::MemberCall {
                object,
                method,
                args,
            } => (object, method, Some(args)),
            _ => {
                return Err(Diagnostic::new(
                    "Expected member assignment target",
                    Some(target_span),
                ));
            }
        };
        if let Some(args) = args {
            return Ok(Stmt::MemberSubCall {
                object: *object,
                method: field,
                args,
                span: target_span,
            });
        }
        self.expect_simple(TokenKind::Equal, "Expected '=' in member assignment")?;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Assign {
            target: AssignTarget::Member {
                object: *object,
                field,
                span: target_span,
            },
            expr,
            span: Span::new(target_span.start, end.end),
        })
    }

    fn parse_assignment_target(&mut self) -> Result<AssignTarget, Diagnostic> {
        let expr = self.parse_primary()?;
        let span = expr.span;
        match expr.kind {
            ExprKind::Variable(name) => Ok(AssignTarget::Variable { name, span }),
            ExprKind::Call { name, args } => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "Array assignment requires exactly one index",
                        Some(span),
                    ));
                }
                let mut args = args.into_iter();
                Ok(AssignTarget::ArrayElement {
                    name,
                    index: args.next().expect("len checked"),
                    span,
                })
            }
            ExprKind::MemberAccess { object, field } => Ok(AssignTarget::Member {
                object: *object,
                field,
                span,
            }),
            ExprKind::Me => Err(Diagnostic::new("Me is not assignable", Some(span))),
            _ => Err(Diagnostic::new("Expected assignment target", Some(span))),
        }
    }

    fn parse_console_writeline(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Console, "Expected 'Console'")?
            .span;
        self.expect_simple(TokenKind::Dot, "Expected '.' after 'Console'")?;
        self.expect_simple(
            TokenKind::WriteLine,
            "Expected 'WriteLine' after 'Console.'",
        )?;
        self.expect_simple(
            TokenKind::LeftParen,
            "Expected '(' after 'Console.WriteLine'",
        )?;

        let mut args = Vec::new();
        if !self.check_simple(&TokenKind::RightParen) {
            loop {
                args.push(self.parse_expression()?);
                if !self.match_simple(&TokenKind::Comma) {
                    break;
                }
            }
        }

        let end = self
            .expect_simple(TokenKind::RightParen, "Expected ')' after arguments")?
            .span;
        Ok(Stmt::ConsoleWriteLine {
            args,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::Return, "Expected 'Return'")?
            .span;
        let expr = self.parse_expression()?;
        let end = expr.span;

        Ok(Stmt::Return {
            expr,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::If, "Expected 'If'")?.span;
        let condition = self.parse_expression()?;
        self.expect_simple(TokenKind::Then, "Expected 'Then' after If condition")?;
        self.expect_newline("Expected newline after 'Then'")?;

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
            span: Span::new(start.start, end.end),
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
            span: Span::new(start.start, end.end),
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
            span: Span::new(start.start, end.end),
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
            span: Span::new(start.start, end.end),
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
            span: Span::new(start.start, end.end),
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
            TokenKind::Identifier(_) => {
                let token = self.advance();
                let TokenKind::Identifier(name) = token.kind else {
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
            span: Span::new(start.start, end.end),
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
            TokenKind::Identifier(_) => {
                let token = self.advance();
                let TokenKind::Identifier(name) = token.kind else {
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
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_redim(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self
            .expect_simple(TokenKind::ReDim, "Expected 'ReDim'")?
            .span;
        let preserve = self.match_simple(&TokenKind::Preserve);
        let name = self.expect_identifier("Expected array name after 'ReDim'")?;
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after array name")?;
        let upper_bound = self.parse_expression()?;
        self.expect_simple(TokenKind::RightParen, "Expected ')' after upper bound")?;
        let end = self.previous().span;
        Ok(Stmt::ReDim {
            name,
            upper_bound,
            preserve,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_goto(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.expect_simple(TokenKind::GoTo, "Expected 'GoTo'")?.span;
        let token = self.advance();
        let label = match token.kind {
            TokenKind::Identifier(label) => label,
            TokenKind::Integer(number) => number.to_string(),
            _ => {
                return Err(Diagnostic::new(
                    "Expected label name after 'GoTo'",
                    Some(token.span),
                ));
            }
        };
        Ok(Stmt::GoTo {
            label,
            span: Span::new(start.start, token.span.end),
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
            let mode = match token.kind {
                TokenKind::Integer(0) => OnErrorMode::GoToZero,
                TokenKind::Minus => {
                    let one = self.advance();
                    if matches!(one.kind, TokenKind::Integer(1)) {
                        OnErrorMode::GoToMinusOne
                    } else {
                        return Err(Diagnostic::new(
                            "On Error GoTo requires 0, -1, or a label",
                            Some(one.span),
                        ));
                    }
                }
                TokenKind::Integer(number) => OnErrorMode::GoToLabel(number.to_string()),
                TokenKind::Identifier(label) => OnErrorMode::GoToLabel(label),
                _ => {
                    return Err(Diagnostic::new(
                        "On Error GoTo requires 0, -1, or a label",
                        Some(token.span),
                    ));
                }
            };
            mode
        } else {
            return Err(self.error_here(
                "Expected 'Resume Next', 'GoTo 0', 'GoTo -1', or 'GoTo <label>' after 'On Error'",
            ));
        };
        let end = self.previous().span;
        Ok(Stmt::OnError {
            mode,
            span: Span::new(start.start, end.end),
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
                    span: Span::new(start.start, next.span.end),
                });
            }
            TokenKind::Identifier(_) => {
                let token = self.advance();
                let TokenKind::Identifier(label) = token.kind else {
                    unreachable!("peek checked");
                };
                return Ok(Stmt::Resume {
                    target: ResumeTarget::Label(label),
                    span: Span::new(start.start, token.span.end),
                });
            }
            TokenKind::Integer(_) => {
                let token = self.advance();
                let TokenKind::Integer(number) = token.kind else {
                    unreachable!("peek checked");
                };
                return Ok(Stmt::Resume {
                    target: ResumeTarget::Label(number.to_string()),
                    span: Span::new(start.start, token.span.end),
                });
            }
            _ => ResumeTarget::Retry,
        };
        Ok(Stmt::Resume {
            target,
            span: start,
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
                    "Expected 'Sub', 'Function', 'For', 'While', or 'Do' after 'Exit'",
                    Some(token.span),
                ));
            }
        };
        Ok(Stmt::Exit {
            target,
            span: Span::new(start.start, token.span.end),
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
        ) || (matches!(self.peek_kind(), TokenKind::End)
            && matches!(
                self.peek_next_kind(),
                Some(
                    TokenKind::If
                        | TokenKind::Sub
                        | TokenKind::Function
                        | TokenKind::Property
                        | TokenKind::Select
                        | TokenKind::Type
                        | TokenKind::Enum
                        | TokenKind::Class
                        | TokenKind::With
                )
            ))
    }
}
