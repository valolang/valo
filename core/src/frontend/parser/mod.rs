//! Valo Parser
//!
//! A recursive descent parser that transforms tokens into an Abstract Syntax Tree (AST).

mod declarations;
mod expressions;
mod program;
mod statements;

use crate::frontend::ast::*;
use crate::frontend::lexer::{Lexer, Token, TokenKind};
use crate::frontend::preprocessor::preprocess;
use crate::runtime::{Diagnostic, FileId};

pub fn parse_source(source: &str) -> Result<Program, Diagnostic> {
    Parser::parse_source(source, FileId::default())
}

pub fn parse_source_with_id(source: &str, file_id: FileId) -> Result<Program, Diagnostic> {
    Parser::parse_source(source, file_id)
}

pub struct Parser {
    pub(super) file_id: FileId,
    pub(super) tokens: Vec<Token>,
    pub(super) current: usize,
}

impl Parser {
    pub fn parse_source(source: &str, file_id: FileId) -> Result<Program, Diagnostic> {
        let source = preprocess(source)?;
        let tokens = Lexer::new(&source).with_id(file_id).tokenize()?;
        Self::new(tokens, file_id).parse_program()
    }

    pub fn new(tokens: Vec<Token>, file_id: FileId) -> Self {
        Self {
            file_id,
            tokens,
            current: 0,
        }
    }

    pub(super) fn consume_statement_end(&mut self) {
        if self.check_simple(&TokenKind::Newline) {
            self.skip_newlines();
        }
    }

    pub(super) fn expect_statement_end(&mut self, message: &str) -> Result<(), Diagnostic> {
        if self.check_simple(&TokenKind::Newline)
            || self.check_simple(&TokenKind::Colon)
            || self.check_simple(&TokenKind::Eof)
            || self.matches_any_block_boundary()
        {
            if self.check_simple(&TokenKind::Colon) {
                self.advance();
                return Ok(());
            }
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
            TokenKind::Version => Ok("VERSION".to_string()),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                message,
                Some(token.span),
            )
            .with_primary_label(message)),
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
        Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            message,
            Some(self.peek().span),
        )
        .with_primary_label(message)
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
    EndStructure,
    EndEnum,
    EndClass,
    EndWith,
    EndUsing,
    Case,
    Loop,
    Next,
    Wend,
    Catch,
    Finally,
    EndTry,
}

#[cfg(test)]
mod tests;
