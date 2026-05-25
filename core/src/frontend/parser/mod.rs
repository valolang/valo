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
use std::collections::HashMap;

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
    pub(super) pending_generic_constraints: HashMap<String, GenericParamConstraint>,
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
            pending_generic_constraints: HashMap::new(),
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
            TokenKind::Identifier(name, _) => Ok(name),
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

    pub(super) fn match_identifier(&mut self, expected: &str) -> bool {
        if matches!(self.peek_kind(), TokenKind::Identifier(name, _) if name.eq_ignore_ascii_case(expected))
        {
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
            crate::runtime::DiagnosticCode::PARSE,
            message,
            Some(self.peek().span),
        )
        .with_primary_label(message)
        .with_note(format!("found {}", token_description(self.peek_kind())))
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

    pub(super) fn expect_identifier_with(
        &mut self,
        expected: &str,
        message: &str,
    ) -> Result<String, Diagnostic> {
        if self.match_identifier(expected) {
            Ok(expected.to_string())
        } else {
            Err(self.error_here(message))
        }
    }
}

fn token_description(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(name, _) => format!("identifier '{name}'"),
        TokenKind::String(_) => "string literal".to_string(),
        TokenKind::Integer(_) => "integer literal".to_string(),
        TokenKind::Float(_) => "number literal".to_string(),
        TokenKind::Eof => "end of file".to_string(),
        TokenKind::Newline => "newline".to_string(),
        TokenKind::LeftParen => "'('".to_string(),
        TokenKind::RightParen => "')'".to_string(),
        TokenKind::Comma => "','".to_string(),
        TokenKind::Dot => "'.'".to_string(),
        TokenKind::Equal => "'='".to_string(),
        TokenKind::End => "'End'".to_string(),
        TokenKind::If => "'If'".to_string(),
        TokenKind::Then => "'Then'".to_string(),
        TokenKind::Import => "'Import'".to_string(),
        TokenKind::Namespace => "'Namespace'".to_string(),
        TokenKind::Module => "'Module'".to_string(),
        TokenKind::As => "'As'".to_string(),
        other => format!("{other:?}"),
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
    EndGet,
    EndSet,
    EndType,
    EndStructure,
    EndInterface,
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

#[cfg(test)]
mod multiline_test;
