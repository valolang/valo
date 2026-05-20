//! Valo Core
//!
//! This crate contains the core logic for the Valo language, divided into three primary layers:
//!
//! 1. **Frontend**: Lexer, Parser, AST, Semantics, and Module loading.
//! 2. **Runtime**: Core value system, type names, and diagnostics.
//! 3. **Backend**: The execution engine (currently a tree-walking interpreter).

pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod modules;
pub mod parser;
pub mod preprocessor;
pub mod runtime;
pub mod semantics;

pub use ast::*;
pub use interpreter::{Frame, Interpreter, run};
pub use lexer::{Lexer, Token, TokenKind};
pub use modules::{Project, load_project};
pub use parser::Parser;
pub use runtime::{Diagnostic, ObjectValue, SourcePos, Span, TypeName, Value};
pub use semantics::validate;

pub fn parse_source(source: &str) -> Result<Program, Diagnostic> {
    Parser::parse_source(source)
}

pub fn run_source(source: &str) -> Result<Vec<String>, Diagnostic> {
    let program = parse_source(source)?;
    validate(&program)?;
    run(&program)
}

pub fn run_file(path: impl AsRef<std::path::Path>) -> Result<Vec<String>, Diagnostic> {
    let project = load_project(path)?;
    semantics::validate_project(&project)?;
    Interpreter::new().run_project(&project)
}
