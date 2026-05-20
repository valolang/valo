//! Valo Core
//!
//! This crate contains the core logic for the Valo language, divided into three primary layers:
//!
//! 1. **Frontend**: Lexer, Parser, AST, Semantics, and Module loading.
//! 2. **Runtime**: Core value system, type names, and diagnostics.
//! 3. **Backend**: The execution engine (currently a tree-walking interpreter).

pub mod backend;
pub mod frontend;
pub mod runtime;

// Re-exports for compatibility and ease of use
pub use backend::interpreter::{self, run, Frame, Interpreter};
pub use frontend::ast::*;
pub use frontend::lexer::{self, Lexer, Token, TokenKind};
pub use frontend::modules::{self, load_project, Project};
pub use frontend::parser::{self, parse_source, Parser};
pub use frontend::preprocessor;
pub use frontend::semantics::{self, validate, validate_project};
pub use runtime::*;

// Top-level convenience functions
pub use runtime::Diagnostic;

pub fn run_source(source: &str) -> Result<Vec<String>, Diagnostic> {
    let program = parse_source(source)?;
    semantics::validate(&program)?;
    run(&program)
}

pub fn run_file(path: impl AsRef<std::path::Path>) -> Result<Vec<String>, Diagnostic> {
    let project = load_project(path)?;
    semantics::validate_project(&project)?;
    Interpreter::new().run_project(&project)
}
