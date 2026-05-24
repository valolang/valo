use super::*;
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
        let mut expr = self.parse_or()?;

        while self.match_simple(&TokenKind::Xor) {
            let right = self.parse_or()?;
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

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_and()?;

        while self.match_simple(&TokenKind::Or) {
            let right = self.parse_and()?;
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
        let mut expr = self.parse_concat()?;

        while let Some(op) = self.match_comparison_op() {
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

            let right = self.parse_power()?;
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
        let token = self.advance();
        let span = token.span;
        let kind = match token.kind {
            TokenKind::String(value) => ExprKind::String(value),
            TokenKind::Integer(value) => ExprKind::Integer(value),
            TokenKind::Hex(value) => ExprKind::Integer(parse_vba_hex(&value)),
            TokenKind::Octal(value) => ExprKind::Integer(parse_vba_octal(&value)),
            TokenKind::Float(value) => parse_vba_float(&value),
            TokenKind::True => ExprKind::Boolean(true),
            TokenKind::False => ExprKind::Boolean(false),
            TokenKind::Nothing => ExprKind::Nothing,
            TokenKind::Empty => ExprKind::Empty,
            TokenKind::Null => ExprKind::Null,
            TokenKind::Me => ExprKind::Me,
            TokenKind::Dot => {
                let field_token = self.advance();
                let field = match field_token.kind {
                    TokenKind::Identifier(field, _) => field,
                    TokenKind::Version => "VERSION".to_string(),
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "Expected member name after '.'",
                            Some(field_token.span),
                        ));
                    }
                };
                let object = Expr {
                    kind: ExprKind::WithTarget,
                    span,
                };
                let member_span = Span::new(self.file_id, span.start, field_token.span.end);
                if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    let end = self.previous().span;
                    return self.parse_member_access(Expr {
                        kind: ExprKind::MemberCall {
                            object: Box::new(object),
                            method: field,
                            type_args: Vec::new(),
                            args,
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    });
                }
                return self.parse_member_access(Expr {
                    kind: ExprKind::MemberAccess {
                        object: Box::new(object),
                        field,
                    },
                    span: member_span,
                });
            }
            TokenKind::New => {
                let mut class_name = self.expect_identifier("Expected class name after 'New'")?;
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
                    crate::runtime::TypeName::User(class_name)
                };
                let args = if self.match_simple(&TokenKind::LeftParen) {
                    self.finish_call_arguments()?
                } else {
                    Vec::new()
                };
                ExprKind::New { class_name, args }
            }
            TokenKind::Identifier(name, _) => {
                if name.eq_ignore_ascii_case("MyBase") {
                    return self.parse_member_access(Expr {
                        kind: ExprKind::MyBase,
                        span,
                    });
                }
                if name.eq_ignore_ascii_case("MyClass") {
                    return self.parse_member_access(Expr {
                        kind: ExprKind::MyClass,
                        span,
                    });
                }
                if name.eq_ignore_ascii_case("iif") && self.match_simple(&TokenKind::LeftParen) {
                    let condition = self.parse_expression()?;
                    self.expect_simple(TokenKind::Comma, "Expected ',' in IIf")?;
                    let true_expr = self.parse_expression()?;
                    self.expect_simple(TokenKind::Comma, "Expected ',' in IIf")?;
                    let false_expr = self.parse_expression()?;
                    self.expect_simple(TokenKind::RightParen, "Expected ')' after IIf")?;
                    let end = self.previous().span;
                    return self.parse_member_access(Expr {
                        kind: ExprKind::IIf {
                            condition: Box::new(condition),
                            true_expr: Box::new(true_expr),
                            false_expr: Box::new(false_expr),
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    });
                }

                if self.check_simple(&TokenKind::LeftParen)
                    && matches!(self.peek_next_kind(), Some(TokenKind::Of))
                {
                    let type_args = self.parse_optional_type_args()?;
                    self.expect_simple(TokenKind::LeftParen, "Expected '(' after type arguments")?;
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
                if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    ExprKind::Call {
                        name,
                        type_args: Vec::new(),
                        args,
                    }
                } else {
                    ExprKind::Variable(name)
                }
            }
            TokenKind::LeftParen => {
                let expr = self.parse_expression()?;
                self.expect_simple(TokenKind::RightParen, "Expected ')' after expression")?;
                return Ok(expr);
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    "Expected expression",
                    Some(span),
                ));
            }
        };

        let expr = Expr { kind, span };
        self.parse_member_access(expr)
    }

    pub(super) fn parse_member_access(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
        loop {
            if self.match_simple(&TokenKind::Dot) {
                let field_token = self.advance();
                let field = match field_token.kind {
                    TokenKind::Identifier(field, _) => field,
                    TokenKind::Version => "VERSION".to_string(),
                    TokenKind::WriteLine => "WriteLine".to_string(),
                    _ => {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PARSE,
                            "Expected field name after '.'",
                            Some(field_token.span),
                        ));
                    }
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
                        },
                        span: Span::new(self.file_id, span.start, end.end),
                    };
                } else {
                    expr = Expr {
                        kind: ExprKind::MemberAccess {
                            object: Box::new(expr),
                            field,
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

    pub(super) fn parse_call_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after name")?;
        self.finish_call_arguments()
    }

    pub(super) fn finish_call_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        let mut saw_named = false;
        if !self.check_simple(&TokenKind::RightParen) {
            loop {
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
        }
        self.expect_simple(TokenKind::RightParen, "Expected ')' after arguments")?;
        Ok(args)
    }

    pub(super) fn parse_argument(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek_kind(), TokenKind::Identifier(_, _))
            && matches!(self.peek_next_kind(), Some(TokenKind::Colon))
        {
            let name_token = self.advance();
            let TokenKind::Identifier(name, _) = name_token.kind else {
                unreachable!("peek checked");
            };
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

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Equal => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::Less,
            TokenKind::Greater => BinaryOp::Greater,
            TokenKind::LessEqual => BinaryOp::LessEqual,
            TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
            TokenKind::Is => BinaryOp::Is,
            TokenKind::Like => BinaryOp::Like,
            _ => return None,
        };
        self.advance();
        Some(op)
    }
}

fn parse_vba_hex(text: &str) -> i64 {
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

fn parse_vba_octal(text: &str) -> i64 {
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

fn parse_vba_float(text: &str) -> ExprKind {
    let mut s = text.to_ascii_lowercase();
    let suffix = s.chars().last();
    match suffix {
        Some('%') => {
            s.pop();
            ExprKind::Integer(s.parse::<i16>().map_or(0, |v| v as i64))
        }
        Some('&') => {
            s.pop();
            ExprKind::Long(s.parse::<i32>().unwrap_or(0))
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
