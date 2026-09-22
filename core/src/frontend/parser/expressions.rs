use super::*;
use crate::frontend::lexer::InterpolationSegment;
use crate::runtime::{Diagnostic, Span};

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_imp()
    }

    fn parse_imp(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_eqv()?;

        while self.match_simple(&TokenKind::Imp) {
            let right = self.parse_eqv()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalImp,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_eqv(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_xor()?;

        while self.match_simple(&TokenKind::Eqv) {
            let right = self.parse_xor()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalEqv,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_xor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_orelse()?;

        while self.match_simple(&TokenKind::Xor) {
            let right = self.parse_orelse()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalXor,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_orelse(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_or()?;

        while self.match_simple(&TokenKind::OrElse) {
            let right = self.parse_or()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalOrElse,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_andalso()?;

        while self.match_simple(&TokenKind::Or) {
            let right = self.parse_andalso()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalOr,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_andalso(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_and()?;

        while self.match_simple(&TokenKind::AndAlso) {
            let right = self.parse_and()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalAndAlso,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_not()?;

        while self.match_simple(&TokenKind::And) {
            let right = self.parse_not()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::LogicalAnd,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_not(&mut self) -> Result<Expr, Diagnostic> {
        if self.match_simple(&TokenKind::TypeOf) {
            let start = self.previous().span;
            let expr = self.parse_concat()?;
            self.expect_simple(TokenKind::Is, "Expected 'Is' after TypeOf expression")?;
            let class_name = self.expect_identifier("Expected class name after 'TypeOf ... Is'")?;
            let end = self.previous().span;
            return Ok(Expr {
                kind: ExprKind::TypeOfIs {
                    expr: Box::new(expr),
                    class_name,
                },
                span: Span::new(self.file_id, start.start, end.end),
            });
        }
        if self.match_simple(&TokenKind::Not) {
            let start = self.previous().span;
            let expr = self.parse_not()?;
            let span = Span::new(self.file_id, start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::LogicalNot,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_shift()?;

        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_shift()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    /// Parses `<<` and `>>`, which bind tighter than comparison and looser than
    /// concatenation, matching VB.NET operator precedence.
    fn parse_shift(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_concat()?;

        loop {
            let op = if self.match_simple(&TokenKind::ShiftLeft) {
                BinaryOp::ShiftLeft
            } else if self.match_simple(&TokenKind::ShiftRight) {
                BinaryOp::ShiftRight
            } else {
                break;
            };
            let right = self.parse_concat()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_concat(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_term()?;

        while self.match_simple(&TokenKind::Ampersand) {
            let right = self.parse_term()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Concat,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_factor()?;

        loop {
            let op = if self.match_simple(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_simple(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_factor()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = if self.match_simple(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.match_simple(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else if self.match_simple(&TokenKind::Backslash) {
                Some(BinaryOp::IntegerDivide)
            } else if self.match_simple(&TokenKind::Mod) {
                Some(BinaryOp::Modulo)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_unary()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            expr = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expr, Diagnostic> {
        let expr = self.parse_primary()?;
        if self.match_simple(&TokenKind::Caret) {
            let right = self.parse_unary()?;
            let span = Span::new(self.file_id, expr.span.start, right.span.end);
            return Ok(Expr {
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Exponent,
                    right: Box::new(right),
                },
                span,
            });
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.match_simple(&TokenKind::Plus) {
            let start = self.previous().span;
            let expr = self.parse_unary()?;
            let span = Span::new(self.file_id, start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Positive,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        if self.match_simple(&TokenKind::Minus) {
            let start = self.previous().span;
            let expr = self.parse_unary()?;
            let span = Span::new(self.file_id, start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        if self.match_simple(&TokenKind::AddressOf) {
            let start = self.previous().span;
            let expr = self.parse_primary()?;
            let span = Span::new(self.file_id, start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::AddressOf(Box::new(expr)),
                span,
            });
        }

        if self.match_simple(&TokenKind::ByVal) {
            let start = self.previous().span;
            let expr = self.parse_primary()?;
            let span = Span::new(self.file_id, start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::PassingModeOverride {
                    mode: crate::frontend::ast::PassingMode::ByVal,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        if self.match_simple(&TokenKind::ByRef) {
            let start = self.previous().span;
            let expr = self.parse_primary()?;
            let span = Span::new(self.file_id, start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::PassingModeOverride {
                    mode: crate::frontend::ast::PassingMode::ByRef,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        self.parse_power()
    }

    pub(super) fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        // A query has to be recognised before the first token is taken: it is
        // `From` plus what follows that makes one, and `From` on its own is an
        // ordinary name.
        if self.at_query_start() {
            return self.parse_query();
        }
        let token = self.advance();
        let span = token.span;
        let expr = match token.kind {
            TokenKind::String(value) => Expr {
                kind: ExprKind::String(value),
                span,
            },
            TokenKind::InterpolatedString(segments) => Expr {
                kind: ExprKind::Interpolated(self.parse_interpolation_parts(segments, span)?),
                span,
            },
            TokenKind::Integer(value) => Expr {
                kind: ExprKind::Integer(value),
                span,
            },
            TokenKind::Hex(value) => Expr {
                kind: ExprKind::Integer(parse_hex_literal(&value)),
                span,
            },
            TokenKind::Octal(value) => Expr {
                kind: ExprKind::Integer(parse_octal_literal(&value)),
                span,
            },
            TokenKind::Float(value) => Expr {
                kind: parse_numeric_literal(&value),
                span,
            },
            TokenKind::True => Expr {
                kind: ExprKind::Boolean(true),
                span,
            },
            TokenKind::False => Expr {
                kind: ExprKind::Boolean(false),
                span,
            },
            TokenKind::Hash => Expr {
                kind: self.parse_date_literal(span)?,
                span: Span::new(self.file_id, span.start, self.previous().span.end),
            },
            TokenKind::Function => self.parse_lambda(span, false)?,
            TokenKind::Sub => self.parse_lambda(span, true)?,
            TokenKind::Await => {
                let expr = self.parse_expression()?;
                Expr {
                    kind: ExprKind::Await(Box::new(expr)),
                    span: Span::new(self.file_id, span.start, self.previous().span.end),
                }
            }
            TokenKind::Nothing => Expr {
                kind: ExprKind::Nothing,
                span,
            },
            TokenKind::Empty => Expr {
                kind: ExprKind::Empty,
                span,
            },
            TokenKind::Null => Expr {
                kind: ExprKind::Null,
                span,
            },
            TokenKind::Me => Expr {
                kind: ExprKind::Me,
                span,
            },
            TokenKind::Dot => {
                let start_span = span;
                let field_token = self.peek();
                let (field, field_end) = match &field_token.kind {
                    TokenKind::Identifier(field, _) => (field.clone(), field_token.span.end),
                    TokenKind::Version => ("VERSION".to_string(), field_token.span.end),
                    TokenKind::WriteLine => ("WriteLine".to_string(), field_token.span.end),
                    TokenKind::Text => ("Text".to_string(), field_token.span.end),
                    TokenKind::Binary => ("Binary".to_string(), field_token.span.end),
                    TokenKind::Compare => ("Compare".to_string(), field_token.span.end),
                    TokenKind::Base => ("Base".to_string(), field_token.span.end),
                    TokenKind::Lib => ("Lib".to_string(), field_token.span.end),
                    TokenKind::New => ("New".to_string(), field_token.span.end),
                    TokenKind::Type => ("Type".to_string(), field_token.span.end),
                    TokenKind::Class => ("Class".to_string(), field_token.span.end),
                    TokenKind::Module => ("Module".to_string(), field_token.span.end),
                    TokenKind::Enum => ("Enum".to_string(), field_token.span.end),
                    TokenKind::Interface => ("Interface".to_string(), field_token.span.end),
                    TokenKind::Structure => ("Structure".to_string(), field_token.span.end),
                    TokenKind::Get => ("Get".to_string(), field_token.span.end),
                    TokenKind::Let => ("Let".to_string(), field_token.span.end),
                    TokenKind::Set => ("Set".to_string(), field_token.span.end),
                    TokenKind::Option => ("Option".to_string(), field_token.span.end),
                    TokenKind::Explicit => ("Explicit".to_string(), field_token.span.end),
                    TokenKind::Sub => ("Sub".to_string(), field_token.span.end),
                    TokenKind::Function => ("Function".to_string(), field_token.span.end),
                    TokenKind::Property => ("Property".to_string(), field_token.span.end),
                    TokenKind::Event => ("Event".to_string(), field_token.span.end),
                    TokenKind::Declare => ("Declare".to_string(), field_token.span.end),
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "Expected member name after '.'",
                            Some(field_token.span),
                        ));
                    }
                };
                self.advance();
                let object = Expr {
                    kind: ExprKind::WithTarget,
                    span: start_span,
                };
                let member_span = Span::new(self.file_id, start_span.start, field_end);
                if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    let end = self.previous().span;
                    Expr {
                        kind: ExprKind::MemberCall {
                            object: Box::new(object),
                            method: field,
                            type_args: Vec::new(),
                            args,
                            conditional: false,
                        },
                        span: Span::new(self.file_id, start_span.start, end.end),
                    }
                } else {
                    Expr {
                        kind: ExprKind::MemberAccess {
                            object: Box::new(object),
                            field,
                            conditional: false,
                        },
                        span: member_span,
                    }
                }
            }
            TokenKind::New if self.check_simple(&TokenKind::With) => {
                // `New With { .X = 1 }` names no type, so there is none to
                // construct. It is a value with named members, which is what a
                // named-element tuple already is, so it becomes one, and
                // member access, copying, and printing all follow from that.
                let Some(inits) = self.parse_object_initializer_allowing_key(true)? else {
                    return Err(self.error_here("Expected '{' after 'New With'"));
                };
                let elements = inits
                    .into_iter()
                    .map(|init| TupleElementExpr {
                        name: Some(init.name),
                        value: init.value,
                    })
                    .collect();
                Expr {
                    kind: ExprKind::TupleLiteral(elements),
                    span: Span::new(self.file_id, span.start, self.previous().span.end),
                }
            }
            TokenKind::New => {
                let mut class_name = if self.match_simple(&TokenKind::Error) {
                    "Error".to_string()
                } else if self.match_simple(&TokenKind::Collection) {
                    "Collection".to_string()
                } else {
                    self.expect_identifier("Expected class name after 'New'")?
                };
                if self.match_simple(&TokenKind::Dot) {
                    let member =
                        self.expect_identifier("Expected class name after module qualifier")?;
                    class_name.push('.');
                    class_name.push_str(&member);
                }
                let class_name = if self.check_simple(&TokenKind::LeftParen)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                {
                    self.parse_generic_type_instance(class_name)?
                } else {
                    crate::frontend::type_model::TypeName::User(class_name)
                };
                let args = if self.match_simple(&TokenKind::LeftParen) {
                    self.finish_call_arguments()?
                } else {
                    Vec::new()
                };
                let mut initializer = None;
                if self.match_simple(&TokenKind::From) {
                    self.expect_simple(
                        TokenKind::LeftBrace,
                        "Expected '{' after 'From' in collection initializer",
                    )?;
                    let mut init_args = Vec::new();
                    if !self.check_simple(&TokenKind::RightBrace) {
                        loop {
                            init_args.push(self.parse_expression()?);
                            if !self.match_simple(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect_simple(
                        TokenKind::RightBrace,
                        "Expected '}' after collection initializer",
                    )?;
                    initializer = Some(init_args);
                }
                let member_initializer = self.parse_object_initializer()?;
                let end = self.previous().span;
                Expr {
                    kind: ExprKind::New {
                        class_name,
                        args,
                        initializer,
                        member_initializer,
                    },
                    span: Span::new(self.file_id, span.start, end.end),
                }
            }
            TokenKind::StringType => {
                let name = "String".to_string();
                let kind = if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    ExprKind::Call {
                        name,
                        type_args: Vec::new(),
                        args,
                    }
                } else {
                    ExprKind::Variable(name)
                };
                let end = self.previous().span;
                Expr {
                    kind,
                    span: Span::new(self.file_id, span.start, end.end),
                }
            }
            TokenKind::Error => {
                let name = "Error".to_string();
                let kind = if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    ExprKind::Call {
                        name,
                        type_args: Vec::new(),
                        args,
                    }
                } else {
                    ExprKind::Variable(name)
                };
                let end = self.previous().span;
                Expr {
                    kind,
                    span: Span::new(self.file_id, span.start, end.end),
                }
            }
            TokenKind::Identifier(name, _) => {
                if name.eq_ignore_ascii_case("MyBase") {
                    Expr {
                        kind: ExprKind::MyBase,
                        span,
                    }
                } else if name.eq_ignore_ascii_case("MyClass") {
                    Expr {
                        kind: ExprKind::MyClass,
                        span,
                    }
                } else if let Some(kind) = conversion_kind(&name)
                    && self.check_simple(&TokenKind::LeftParen)
                {
                    self.advance();
                    let value = self.parse_expression()?;
                    self.expect_simple(TokenKind::Comma, "Expected ',' before the target type")?;
                    let target = self.parse_type_name()?;
                    self.expect_simple(
                        TokenKind::RightParen,
                        "Expected ')' after the target type",
                    )?;
                    let end = self.previous().span;
                    Expr {
                        kind: ExprKind::Convert {
                            expr: Box::new(value),
                            target,
                            kind,
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    }
                } else if name.eq_ignore_ascii_case("GetType")
                    && self.check_simple(&TokenKind::LeftParen)
                {
                    self.advance();
                    let target = self.parse_type_name()?;
                    self.expect_simple(TokenKind::RightParen, "Expected ')' after GetType")?;
                    let end = self.previous().span;
                    Expr {
                        kind: ExprKind::GetType(target),
                        span: Span::new(self.file_id, span.start, end.end),
                    }
                } else if name.eq_ignore_ascii_case("NameOf")
                    && self.check_simple(&TokenKind::LeftParen)
                {
                    self.advance();
                    let operand = self.parse_expression()?;
                    self.expect_simple(TokenKind::RightParen, "Expected ')' after NameOf")?;
                    let end = self.previous().span;
                    let Some(name) = source_name_of(&operand) else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "NameOf requires a variable, member, or type name",
                            Some(operand.span),
                        ));
                    };
                    Expr {
                        kind: ExprKind::NameOf(name),
                        span: Span::new(self.file_id, span.start, end.end),
                    }
                } else if name.eq_ignore_ascii_case("iif")
                    && self.match_simple(&TokenKind::LeftParen)
                {
                    let condition = self.parse_expression()?;
                    self.expect_simple(TokenKind::Comma, "Expected ',' in IIf")?;
                    let true_expr = self.parse_expression()?;
                    self.expect_simple(TokenKind::Comma, "Expected ',' in IIf")?;
                    let false_expr = self.parse_expression()?;
                    self.expect_simple(TokenKind::RightParen, "Expected ')' after IIf")?;
                    let end = self.previous().span;
                    Expr {
                        kind: ExprKind::IIf {
                            condition: Box::new(condition),
                            true_expr: Box::new(true_expr),
                            false_expr: Box::new(false_expr),
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    }
                } else {
                    let kind = if self.check_simple(&TokenKind::LeftParen)
                        && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                    {
                        let type_args = self.parse_optional_type_args()?;
                        self.expect_simple(
                            TokenKind::LeftParen,
                            "Expected '(' after type arguments",
                        )?;
                        let args = self.finish_call_arguments()?;
                        ExprKind::Call {
                            name,
                            type_args,
                            args,
                        }
                    } else if self.match_simple(&TokenKind::LeftParen) {
                        let args = self.finish_call_arguments()?;
                        ExprKind::Call {
                            name,
                            type_args: Vec::new(),
                            args,
                        }
                    } else {
                        ExprKind::Variable(name)
                    };
                    let end = self.previous().span;
                    Expr {
                        kind,
                        span: Span::new(self.file_id, span.start, end.end),
                    }
                }
            }
            TokenKind::Console => {
                self.expect_simple(TokenKind::Dot, "Expected '.' after 'Console'")?;
                let method = self.expect_identifier("Expected method name after 'Console.'")?;
                let args = if self.match_simple(&TokenKind::LeftParen) {
                    self.finish_call_arguments()?
                } else {
                    Vec::new()
                };
                let end = self.previous().span;
                Expr {
                    kind: ExprKind::Call {
                        name: format!("Console.{}", method),
                        type_args: Vec::new(),
                        args,
                    },
                    span: Span::new(self.file_id, span.start, end.end),
                }
            }
            TokenKind::Lib
            | TokenKind::Base
            | TokenKind::Text
            | TokenKind::Compare
            | TokenKind::Binary => {
                let name = match token.kind {
                    TokenKind::Lib => "lib".to_string(),
                    TokenKind::Base => "base".to_string(),
                    TokenKind::Text => "text".to_string(),
                    TokenKind::Compare => "compare".to_string(),
                    TokenKind::Binary => "binary".to_string(),
                    _ => unreachable!(),
                };
                let kind = if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    ExprKind::Call {
                        name,
                        type_args: Vec::new(),
                        args,
                    }
                } else {
                    ExprKind::Variable(name)
                };
                let end = self.previous().span;
                Expr {
                    kind,
                    span: Span::new(self.file_id, span.start, end.end),
                }
            }
            TokenKind::LeftParen => {
                // `(x)` groups; `(x, y)` builds a tuple. Only the comma tells
                // them apart, so the first element is parsed either way and the
                // decision is made when it is done.
                let first = self.parse_tuple_element()?;
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    let mut elements = vec![first];
                    while self.match_simple(&TokenKind::Comma) {
                        elements.push(self.parse_tuple_element()?);
                    }
                    self.expect_simple(TokenKind::RightParen, "Expected ')' after tuple")?;
                    Expr {
                        kind: ExprKind::TupleLiteral(elements),
                        span: Span::new(self.file_id, span.start, self.previous().span.end),
                    }
                } else {
                    self.expect_simple(TokenKind::RightParen, "Expected ')' after expression")?;
                    // Fall through rather than returning, so a member access can
                    // continue from here: `(a + b).Describe()` reads the same as
                    // any other receiver.
                    first.value
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected expression",
                    Some(span),
                ));
            }
        };

        self.parse_member_access(expr)
    }

    fn parse_date_literal(&mut self, start: Span) -> Result<ExprKind, Diagnostic> {
        let mut text = String::new();
        while !self.is_at_end() && !self.check_simple(&TokenKind::Hash) {
            let token = self.advance();
            match token.kind {
                TokenKind::Integer(value) => text.push_str(&value.to_string()),
                TokenKind::Float(value) if value.ends_with('#') => {
                    text.push_str(value.trim_end_matches('#'));
                    if text.trim().is_empty() {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "Date literal cannot be empty",
                            Some(start),
                        ));
                    }
                    return Ok(ExprKind::DateLiteral(text));
                }
                TokenKind::Float(value) => text.push_str(&value),
                TokenKind::Slash => text.push('/'),
                TokenKind::Minus => text.push('-'),
                TokenKind::Colon => text.push(':'),
                TokenKind::Identifier(value, _) => text.push_str(&value),
                TokenKind::String(value) => text.push_str(&value),
                _ => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::PARSE,
                        "Invalid date literal",
                        Some(token.span),
                    ));
                }
            }
        }
        self.expect_simple(TokenKind::Hash, "Expected '#' after date literal")?;
        if text.trim().is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Date literal cannot be empty",
                Some(start),
            ));
        }
        Ok(ExprKind::DateLiteral(text))
    }

    pub(super) fn parse_member_access(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
        // Once a `?.` appears, the rest of the chain is guarded too: in
        // `a?.B.C`, a `Nothing` receiver makes the whole expression Nothing
        // rather than failing on `.C`.
        let mut conditional = false;
        loop {
            if self.check_simple(&TokenKind::Question) && self.check_next_simple(&TokenKind::Dot) {
                self.advance();
                conditional = true;
            }
            if self.match_simple(&TokenKind::Dot) {
                let field_token = self.advance();
                let Some(field) = contextual_identifier_name(&field_token.kind) else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::PARSE,
                        "Expected field name after '.'",
                        Some(field_token.span),
                    ));
                };
                let span = Span::new(self.file_id, expr.span.start, field_token.span.end);
                if self.check_simple(&TokenKind::LeftParen)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                {
                    let type_args = self.parse_optional_type_args()?;
                    self.expect_simple(TokenKind::LeftParen, "Expected '(' after type arguments")?;
                    let args = self.finish_call_arguments()?;
                    let end = self.previous().span;
                    expr = Expr {
                        kind: ExprKind::MemberCall {
                            object: Box::new(expr),
                            method: field,
                            type_args,
                            args,
                            conditional,
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    };
                } else if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    let end = self.previous().span;
                    expr = Expr {
                        kind: ExprKind::MemberCall {
                            object: Box::new(expr),
                            method: field,
                            type_args: Vec::new(),
                            args,
                            conditional,
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    };
                } else {
                    expr = Expr {
                        kind: ExprKind::MemberAccess {
                            object: Box::new(expr),
                            field,
                            conditional,
                        },
                        span,
                    };
                }
            } else if self.match_simple(&TokenKind::LeftParen) {
                let args = self.finish_call_arguments()?;
                let end = self.previous().span;
                let start = expr.span.start;
                expr = Expr {
                    kind: ExprKind::Index {
                        target: Box::new(expr),
                        args,
                    },
                    span: Span::new(self.file_id, start, end.end),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    pub(super) fn finish_call_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        let mut saw_named = false;
        if !self.check_simple(&TokenKind::RightParen) {
            loop {
                let arg = if self.check_simple(&TokenKind::Comma)
                    || self.check_simple(&TokenKind::RightParen)
                {
                    // Omitted argument
                    Expr {
                        kind: ExprKind::Missing,
                        span: self.peek().span,
                    }
                } else {
                    let arg = self.parse_argument()?;
                    if matches!(arg.kind, ExprKind::NamedArg { .. }) {
                        saw_named = true;
                    } else if saw_named {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARGUMENT_COUNT,
                            "Positional arguments cannot appear after named arguments",
                            Some(arg.span),
                        ));
                    }
                    arg
                };
                args.push(arg);
                if !self.match_simple(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect_simple(TokenKind::RightParen, "Expected ')' after arguments")?;
        Ok(args)
    }

    /// Whether a query starts here: `From <name> In ...`.
    ///
    /// `From` already ends a `Get`/`Put` statement, so seeing it is not enough
    /// on its own; three tokens tell a query from any other use of the word.
    pub(super) fn at_query_start(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::From)
            && self
                .peek_next_kind()
                .is_some_and(|kind| contextual_identifier_name(kind).is_some())
            && matches!(self.peek_kind_at(2), Some(TokenKind::In))
    }

    /// Parses `From n In source` followed by its clauses.
    fn parse_query(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.advance().span;
        let variable = self.expect_identifier("Expected a range variable after 'From'")?;
        self.expect_simple(TokenKind::In, "Expected 'In' after the range variable")?;
        let source = self.parse_expression()?;

        let mut clauses = Vec::new();
        let mut selected = false;
        while let Some(keyword) = self.peek_query_clause() {
            // After a projection the range variable no longer holds what the
            // query started from, so a clause that reads it has nothing to
            // read. The ones that only reshape the sequence, meaning Distinct,
            // Take, and Skip, are fine either side of it, and `Select x
            // Distinct` is how that is usually written.
            if selected && keyword.reads_the_range_variable() {
                return Err(self.error_here(
                    "this clause reads the range variable, which Select has replaced; put it before the Select",
                ));
            }
            // A clause may sit on its own line, which is how queries are
            // usually written. The newlines are only consumed once a clause is
            // known to follow, so a query at the end of a line still ends there.
            self.skip_newlines();
            match keyword {
                QueryKeyword::Where => {
                    self.advance();
                    clauses.push(QueryClause::Where(self.parse_expression()?));
                }
                QueryKeyword::OrderBy => {
                    self.advance();
                    self.expect_identifier_named("By", "Expected 'By' after 'Order'")?;
                    let key = self.parse_expression()?;
                    let descending = if self.match_identifier("Descending") {
                        true
                    } else {
                        self.match_identifier("Ascending");
                        false
                    };
                    clauses.push(QueryClause::OrderBy { key, descending });
                }
                QueryKeyword::Distinct => {
                    self.advance();
                    clauses.push(QueryClause::Distinct);
                }
                QueryKeyword::Take => {
                    self.advance();
                    clauses.push(QueryClause::Take(self.parse_expression()?));
                }
                QueryKeyword::Skip => {
                    self.advance();
                    clauses.push(QueryClause::Skip(self.parse_expression()?));
                }
                QueryKeyword::Select => {
                    self.advance();
                    clauses.push(QueryClause::Select(self.parse_expression()?));
                    selected = true;
                }
            }
        }

        Ok(Expr {
            kind: ExprKind::Query {
                variable,
                source: Box::new(source),
                clauses,
            },
            span: Span::new(self.file_id, start.start, self.previous().span.end),
        })
    }

    /// The clause that comes next, looking past any newlines before it.
    fn peek_query_clause(&self) -> Option<QueryKeyword> {
        let mut offset = 0;
        while matches!(self.peek_kind_at(offset), Some(TokenKind::Newline)) {
            offset += 1;
        }
        let kind = self.peek_kind_at(offset)?;
        if matches!(kind, TokenKind::Select) {
            return Some(QueryKeyword::Select);
        }
        let TokenKind::Identifier(name, _) = kind else {
            return None;
        };
        match name.to_ascii_lowercase().as_str() {
            "where" => Some(QueryKeyword::Where),
            "order" => Some(QueryKeyword::OrderBy),
            "distinct" => Some(QueryKeyword::Distinct),
            "take" => Some(QueryKeyword::Take),
            "skip" => Some(QueryKeyword::Skip),
            _ => None,
        }
    }

    fn expect_identifier_named(&mut self, expected: &str, message: &str) -> Result<(), Diagnostic> {
        if self.match_identifier(expected) {
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    /// Parses one element of a tuple literal: `1`, or `X := 1`.
    ///
    /// A name is optional and is only there so the element can be read back as
    /// `point.X` instead of `point.Item1`. It uses the same `:=` as a named
    /// argument, which is how VB.NET spells it.
    fn parse_tuple_element(&mut self) -> Result<TupleElementExpr, Diagnostic> {
        if matches!(self.peek_next_kind(), Some(TokenKind::Colon))
            && matches!(self.peek_kind_at(2), Some(TokenKind::Equal))
            && contextual_identifier_name(self.peek_kind()).is_some()
        {
            let name_token = self.advance();
            let name = contextual_identifier_name(&name_token.kind).expect("peek checked");
            self.expect_simple(TokenKind::Colon, "Expected ':' in named tuple element")?;
            self.expect_simple(TokenKind::Equal, "Expected '=' in named tuple element")?;
            return Ok(TupleElementExpr {
                name: Some(name),
                value: self.parse_expression()?,
            });
        }
        Ok(TupleElementExpr {
            name: None,
            value: self.parse_expression()?,
        })
    }

    pub(super) fn parse_argument(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek_next_kind(), Some(TokenKind::Colon))
            && matches!(self.peek_kind_at(2), Some(TokenKind::Equal))
            && contextual_identifier_name(self.peek_kind()).is_some()
        {
            let name_token = self.advance();
            let name = contextual_identifier_name(&name_token.kind).expect("peek checked");
            self.expect_simple(TokenKind::Colon, "Expected ':' in named argument")?;
            self.expect_simple(TokenKind::Equal, "Expected '=' in named argument")?;
            let expr = self.parse_expression()?;
            let span = Span::new(self.file_id, name_token.span.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::NamedArg {
                    name,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        self.parse_expression()
    }

    /// Parses a lambda: `Function(x) x * 2`, or the multi-line form ending in
    /// `End Function` / `End Sub`.
    ///
    /// The two are told apart by what follows the header: a statement separator
    /// means a statement body, anything else means a single expression. A `Sub`
    /// lambda has no result, so it only has the multi-line form.
    fn parse_lambda(&mut self, start: Span, is_sub: bool) -> Result<Expr, Diagnostic> {
        let keyword = if is_sub { "Sub" } else { "Function" };

        let mut params = Vec::new();
        if self.match_simple(&TokenKind::LeftParen) {
            params = self.parse_lambda_parameters()?;
            self.expect_simple(TokenKind::RightParen, "Expected ')' after parameters")?;
        }

        // A lambda may annotate its result type, which the interpreter infers
        // from the returned value, so it is accepted and not otherwise used.
        if !is_sub && self.match_simple(&TokenKind::As) {
            let _ = self.parse_type_name()?;
        }

        let body = if is_sub || self.check_simple(&TokenKind::Newline) {
            self.expect_newline(&format!(
                "Expected newline after the {keyword} lambda header"
            ))?;
            let body = self.parse_block_until(&[BlockEnd::EndFunction, BlockEnd::EndSub])?;
            self.expect_simple(TokenKind::End, &format!("Expected 'End {keyword}'"))?;
            let closing = self.advance();
            let closed_with_sub = matches!(closing.kind, TokenKind::Sub);
            if closed_with_sub != is_sub {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    format!("Expected 'End {keyword}' to close this lambda"),
                    Some(closing.span),
                ));
            }
            LambdaBody::Statements { body, is_sub }
        } else {
            LambdaBody::Expression(Box::new(self.parse_expression()?))
        };

        Ok(Expr {
            kind: ExprKind::Lambda { params, body },
            span: Span::new(self.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parses a `With { .Member = value, ... }` object initializer, if one
    /// follows.
    ///
    /// VB.NET allows the entries to span lines, so newlines are skipped inside
    /// the braces. A trailing comma before `}` is accepted, which keeps
    /// multi-line initializers easy to edit.
    pub(super) fn parse_object_initializer(
        &mut self,
    ) -> Result<Option<Vec<MemberInit>>, Diagnostic> {
        self.parse_object_initializer_allowing_key(false)
    }

    /// The same, optionally accepting VB.NET's `Key` marker.
    ///
    /// `Key` says an anonymous type's member takes part in equality and
    /// hashing. Valo compares these by value throughout, so the word changes
    /// nothing and is accepted so VB.NET source carries over. It is not
    /// accepted in a named type's initializer, where VB.NET does not allow it
    /// either.
    pub(super) fn parse_object_initializer_allowing_key(
        &mut self,
        allow_key: bool,
    ) -> Result<Option<Vec<MemberInit>>, Diagnostic> {
        if !self.check_simple(&TokenKind::With) || !self.check_next_simple(&TokenKind::LeftBrace) {
            return Ok(None);
        }
        self.advance();
        self.advance();

        let mut inits: Vec<MemberInit> = Vec::new();
        self.skip_newlines();
        while !self.check_simple(&TokenKind::RightBrace) {
            if allow_key {
                self.match_identifier("Key");
            }
            let start = self
                .expect_simple(
                    TokenKind::Dot,
                    "Expected '.' before a member name in an object initializer",
                )?
                .span;
            let name = self.expect_identifier("Expected member name after '.'")?;
            self.expect_simple(
                TokenKind::Equal,
                "Expected '=' after the member name in an object initializer",
            )?;
            let value = self.parse_expression()?;
            let span = Span::new(self.file_id, start.start, value.span.end);

            if let Some(previous) = inits
                .iter()
                .find(|init| init.name.eq_ignore_ascii_case(&name))
            {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION,
                    format!("Member '{}' is initialized more than once", previous.name),
                    Some(span),
                ));
            }

            inits.push(MemberInit { name, value, span });

            self.skip_newlines();
            if !self.match_simple(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        self.skip_newlines();
        self.expect_simple(
            TokenKind::RightBrace,
            "Expected '}' after the object initializer",
        )?;

        Ok(Some(inits))
    }

    /// Turns the scanner's interpolation segments into AST parts.
    ///
    /// Each hole carries raw source text, so it is tokenized and parsed here as
    /// a standalone expression. Diagnostics from a hole are re-pointed at the
    /// literal, since the hole's own text has no independent source location.
    fn parse_interpolation_parts(
        &mut self,
        segments: Vec<InterpolationSegment>,
        literal_span: Span,
    ) -> Result<Vec<InterpolationPart>, Diagnostic> {
        let mut parts = Vec::with_capacity(segments.len());
        for segment in segments {
            match segment {
                InterpolationSegment::Literal(text) => {
                    parts.push(InterpolationPart::Literal(text));
                }
                InterpolationSegment::Hole {
                    source,
                    alignment,
                    format,
                    span,
                } => {
                    let expr = self.parse_embedded_expression(&source, span, literal_span)?;
                    parts.push(InterpolationPart::Value {
                        expr: Box::new(expr),
                        alignment,
                        format,
                    });
                }
            }
        }
        Ok(parts)
    }

    /// Parses a standalone expression from source text embedded in a literal.
    fn parse_embedded_expression(
        &mut self,
        source: &str,
        span: Span,
        literal_span: Span,
    ) -> Result<Expr, Diagnostic> {
        let repoint = |error: Diagnostic| -> Diagnostic {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                format!("Invalid interpolation expression: {}", error.message),
                Some(literal_span),
            )
        };

        let tokens = crate::frontend::lexer::Lexer::new(source)
            .with_id(self.file_id)
            .tokenize()
            .map_err(repoint)?;
        let mut parser = Parser::new(tokens, self.file_id);
        let mut expr = parser.parse_expression().map_err(repoint)?;
        if !matches!(parser.peek_kind(), TokenKind::Eof | TokenKind::Newline) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                "Interpolation hole must contain a single expression",
                Some(literal_span),
            ));
        }
        // The embedded tokens were scanned from their own buffer, so their spans
        // do not line up with the enclosing file.
        expr.span = span;
        Ok(expr)
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Equal => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::Less,
            TokenKind::Greater => BinaryOp::Greater,
            TokenKind::LessEqual => BinaryOp::LessEqual,
            TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
            TokenKind::Is => BinaryOp::Is,
            TokenKind::IsNot => BinaryOp::IsNot,
            TokenKind::Like => BinaryOp::Like,
            _ => return None,
        };
        self.advance();
        Some(op)
    }
}

/// Maps `CType`, `DirectCast`, and `TryCast` to the conversion they perform.
///
/// These read like calls but take a type as their second operand, so they are
/// recognized here rather than resolved as ordinary functions.
fn conversion_kind(name: &str) -> Option<ConversionKind> {
    if name.eq_ignore_ascii_case("CType") {
        Some(ConversionKind::Convert)
    } else if name.eq_ignore_ascii_case("DirectCast") {
        Some(ConversionKind::Direct)
    } else if name.eq_ignore_ascii_case("TryCast") {
        Some(ConversionKind::Try)
    } else {
        None
    }
}

/// Returns the name `NameOf` should report for an operand.
///
/// `NameOf(customer.Address)` yields `"Address"`, matching VB.NET, which names
/// the final member rather than the whole path.
fn source_name_of(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Variable(name) => Some(name.clone()),
        ExprKind::MemberAccess { field, .. } => Some(field.clone()),
        ExprKind::Call { name, .. } => Some(name.clone()),
        ExprKind::MemberCall { method, .. } => Some(method.clone()),
        ExprKind::Me => Some("Me".to_string()),
        _ => None,
    }
}

/// A clause keyword, once one has been recognised at a clause boundary.
#[derive(Debug, Clone, Copy)]
enum QueryKeyword {
    Where,
    OrderBy,
    Distinct,
    Take,
    Skip,
    Select,
}

impl QueryKeyword {
    /// Whether the clause evaluates something against the range variable.
    fn reads_the_range_variable(self) -> bool {
        matches!(
            self,
            QueryKeyword::Where | QueryKeyword::OrderBy | QueryKeyword::Select
        )
    }
}

pub(super) fn contextual_identifier_name(kind: &TokenKind) -> Option<String> {
    Some(match kind {
        TokenKind::Identifier(name, _) => name.clone(),
        TokenKind::Version => "VERSION".to_string(),
        TokenKind::WriteLine => "WriteLine".to_string(),
        TokenKind::Text => "Text".to_string(),
        TokenKind::Binary => "Binary".to_string(),
        TokenKind::Compare => "Compare".to_string(),
        TokenKind::Base => "Base".to_string(),
        TokenKind::Lib => "Lib".to_string(),
        TokenKind::New => "New".to_string(),
        TokenKind::Type => "Type".to_string(),
        TokenKind::Class => "Class".to_string(),
        TokenKind::Module => "Module".to_string(),
        TokenKind::Enum => "Enum".to_string(),
        TokenKind::Interface => "Interface".to_string(),
        TokenKind::Structure => "Structure".to_string(),
        TokenKind::Get => "Get".to_string(),
        TokenKind::Let => "Let".to_string(),
        TokenKind::Set => "Set".to_string(),
        TokenKind::Option => "Option".to_string(),
        TokenKind::Explicit => "Explicit".to_string(),
        TokenKind::Sub => "Sub".to_string(),
        TokenKind::Function => "Function".to_string(),
        TokenKind::Property => "Property".to_string(),
        TokenKind::Event => "Event".to_string(),
        TokenKind::Declare => "Declare".to_string(),
        TokenKind::Select => "Select".to_string(),
        TokenKind::Next => "Next".to_string(),
        TokenKind::Exit => "Exit".to_string(),
        TokenKind::Namespace => "Namespace".to_string(),
        TokenKind::Any => "Any".to_string(),
        TokenKind::Error => "Error".to_string(),
        _ => return None,
    })
}

fn parse_hex_literal(text: &str) -> i64 {
    let mut s = text.to_ascii_uppercase();
    let has_long_suffix = s.ends_with('&');
    if has_long_suffix {
        s.pop();
    }

    let val = i64::from_str_radix(&s, 16).unwrap_or(0);

    if !has_long_suffix && s.len() <= 4 {
        if val > 0x7FFF {
            return val - 0x10000;
        }
    } else if (has_long_suffix || s.len() <= 8) && val > 0x7FFFFFFF {
        return val - 0x100000000;
    }

    val
}

fn parse_octal_literal(text: &str) -> i64 {
    let mut s = text.to_ascii_uppercase();
    let has_long_suffix = s.ends_with('&');
    if has_long_suffix {
        s.pop();
    }

    let val = i64::from_str_radix(&s, 8).unwrap_or(0);

    if !has_long_suffix && val <= 0xFFFF {
        if val > 0x7FFF {
            return val - 0x10000;
        }
    } else if (has_long_suffix || val <= 0xFFFFFFFF) && val > 0x7FFFFFFF {
        return val - 0x100000000;
    }

    val
}

fn parse_numeric_literal(text: &str) -> ExprKind {
    let mut s = text.to_ascii_lowercase();
    let suffix = s.chars().last();
    match suffix {
        Some('%') => {
            s.pop();
            ExprKind::Integer(s.parse::<i32>().map_or(0, |v| v as i64))
        }
        Some('&') => {
            s.pop();
            ExprKind::LongLong(s.parse::<i64>().unwrap_or(0))
        }
        Some('^') => {
            s.pop();
            ExprKind::LongLong(s.parse::<i64>().unwrap_or(0))
        }
        Some('!') => {
            s.pop();
            ExprKind::Single(s.parse::<f32>().unwrap_or(0.0))
        }
        Some('#') => {
            s.pop();
            ExprKind::Double(s.parse::<f64>().unwrap_or(0.0))
        }
        Some('@') => {
            s.pop();
            ExprKind::Currency((s.parse::<f64>().unwrap_or(0.0) * 10000.0) as i64)
        }
        _ => ExprKind::Double(s.parse::<f64>().unwrap_or(0.0)),
    }
}
