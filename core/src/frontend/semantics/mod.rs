//! Valo Semantics
//!
//! Performs semantic analysis and validation on the AST, including symbol resolution
//! and type checking.

mod context;
mod symbols;
mod types;
mod validate;

pub use validate::{validate, validate_project};
