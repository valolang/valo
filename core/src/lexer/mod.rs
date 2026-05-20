//! Valo Lexer
//!
//! Converts raw source code into a stream of tokens.

mod scanner;
mod token;

pub use scanner::Lexer;
pub use token::{Token, TokenKind};
