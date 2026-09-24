//! Valo Semantics
//!
//! Performs semantic analysis and validation on the AST, including symbol resolution
//! and type checking.

pub mod arithmetic;
mod context;
pub mod debug_hir;
pub mod hir;
pub mod ids;
pub mod overloads;
pub mod ownership;
mod symbols;
pub mod type_properties;
pub mod typed_hir;
mod types;
mod validate;
pub mod verify_hir;

pub use hir::{ProjectIndex, build_project_index};
pub use ids::{FunctionId, MemberId, ModuleId, SymbolId, TypeId};
pub use validate::{
    lower_function_body, lower_project_function_body, lower_project_procedure_body,
};
pub use validate::{validate, validate_project, validate_project_for_check, validate_snippet};
