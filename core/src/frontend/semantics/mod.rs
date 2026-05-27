//! Valo Semantics
//!
//! Performs semantic analysis and validation on the AST, including symbol resolution
//! and type checking.

mod context;
pub mod hir;
pub mod ids;
mod symbols;
mod types;
mod validate;

pub use hir::{ProjectIndex, build_project_index};
pub use ids::{FunctionId, MemberId, ModuleId, SymbolId, TypeId};
pub use validate::{validate, validate_project, validate_snippet};
