use super::*;
use valo_runtime::{Diagnostic, Span};

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_and()?;

        while self.match_simple(&TokenKind::Or) {
            let right = self.parse_and()?;
            let span = Span::new(expr.span.start, right.span.end);
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
        let mut expr = self.parse_comparison()?;

        while self.match_simple(&TokenKind::And) {
            let right = self.parse_comparison()?;
            let span = Span::new(expr.span.start, right.span.end);
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

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_concat()?;

        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_concat()?;
            let span = Span::new(expr.span.start, right.span.end);
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
            let span = Span::new(expr.span.start, right.span.end);
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
            let span = Span::new(expr.span.start, right.span.end);
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
            } else if self.match_simple(&TokenKind::Mod) {
                Some(BinaryOp::Modulo)
            } else {
                None
            };

            let Some(op) = op else {
                break;
            };

            let right = self.parse_unary()?;
            let span = Span::new(expr.span.start, right.span.end);
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

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.match_simple(&TokenKind::Minus) {
            let start = self.previous().span;
            let expr = self.parse_unary()?;
            let span = Span::new(start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        if self.match_simple(&TokenKind::Not) {
            let start = self.previous().span;
            let expr = self.parse_unary()?;
            let span = Span::new(start.start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::LogicalNot,
                    expr: Box::new(expr),
                },
                span,
            });
        }

        self.parse_primary()
    }

    pub(super) fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.advance();
        let span = token.span;
        let kind = match token.kind {
            TokenKind::String(value) => ExprKind::String(value),
            TokenKind::Integer(value) => ExprKind::Integer(value),
            TokenKind::True => ExprKind::Boolean(true),
            TokenKind::False => ExprKind::Boolean(false),
            TokenKind::Identifier(name) => {
                if self.match_simple(&TokenKind::LeftParen) {
                    let args = self.finish_call_arguments()?;
                    ExprKind::Call { name, args }
                } else {
                    ExprKind::Variable(name)
                }
            }
            TokenKind::LeftParen => {
                let expr = self.parse_expression()?;
                self.expect_simple(TokenKind::RightParen, "Expected ')' after expression")?;
                return Ok(expr);
            }
            _ => return Err(Diagnostic::new("Expected expression", Some(span))),
        };

        let expr = Expr { kind, span };
        self.parse_member_access(expr)
    }

    pub(super) fn parse_member_access(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
        while self.match_simple(&TokenKind::Dot) {
            let field_token = self.advance();
            let TokenKind::Identifier(field) = field_token.kind else {
                return Err(Diagnostic::new(
                    "Expected field name after '.'",
                    Some(field_token.span),
                ));
            };
            let span = Span::new(expr.span.start, field_token.span.end);
            expr = Expr {
                kind: ExprKind::MemberAccess {
                    object: Box::new(expr),
                    field,
                },
                span,
            };
        }

        Ok(expr)
    }

    pub(super) fn parse_call_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        self.expect_simple(TokenKind::LeftParen, "Expected '(' after name")?;
        self.finish_call_arguments()
    }

    pub(super) fn finish_call_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
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
        Ok(args)
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Equal => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::Less,
            TokenKind::Greater => BinaryOp::Greater,
            TokenKind::LessEqual => BinaryOp::LessEqual,
            TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
            _ => return None,
        };
        self.advance();
        Some(op)
    }
}
