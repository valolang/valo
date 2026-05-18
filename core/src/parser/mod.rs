mod declarations;
mod expressions;
mod program;
mod statements;

use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};
use crate::preprocessor::preprocess;
use crate::runtime::Diagnostic;

pub struct Parser {
    pub(super) tokens: Vec<Token>,
    pub(super) current: usize,
}

impl Parser {
    pub fn parse_source(source: &str) -> Result<Program, Diagnostic> {
        let source = preprocess(source)?;
        let tokens = Lexer::new(&source).tokenize()?;
        Self::new(tokens).parse_program()
    }

    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub(super) fn consume_statement_end(&mut self) {
        if self.check_simple(&TokenKind::Newline) {
            self.skip_newlines();
        }
    }

    pub(super) fn expect_statement_end(&mut self, message: &str) -> Result<(), Diagnostic> {
        if self.check_simple(&TokenKind::Newline)
            || self.check_simple(&TokenKind::Eof)
            || self.matches_any_block_boundary()
        {
            self.consume_statement_end();
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    pub(super) fn expect_newline(&mut self, message: &str) -> Result<(), Diagnostic> {
        if self.check_simple(&TokenKind::Newline) || self.check_simple(&TokenKind::Eof) {
            self.skip_newlines();
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    pub(super) fn skip_newlines(&mut self) {
        while self.match_simple(&TokenKind::Newline) {}
    }

    pub(super) fn expect_identifier(&mut self, message: &str) -> Result<String, Diagnostic> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok(name),
            _ => Err(Diagnostic::new(message, Some(token.span)).with_primary_label(message)),
        }
    }

    pub(super) fn expect_simple(
        &mut self,
        kind: TokenKind,
        message: &str,
    ) -> Result<Token, Diagnostic> {
        if self.check_simple(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error_here(message))
        }
    }

    pub(super) fn match_simple(&mut self, kind: &TokenKind) -> bool {
        if self.check_simple(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn check_simple(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    pub(super) fn error_here(&self, message: &str) -> Diagnostic {
        Diagnostic::new(message, Some(self.peek().span)).with_primary_label(message)
    }

    pub(super) fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    pub(super) fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        token
    }

    pub(super) fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    pub(super) fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    pub(super) fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    pub(super) fn peek_next_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.current + 1).map(|token| &token.kind)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BlockEnd {
    Else,
    ElseIf,
    EndIf,
    EndFunction,
    EndProperty,
    EndSelect,
    EndSub,
    EndType,
    EndEnum,
    EndClass,
    EndWith,
    Case,
    Loop,
    Next,
    Wend,
}

#[cfg(test)]
mod tests;
