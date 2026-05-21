//! Valo Backend: Tree-walking Interpreter
//!
//! This module implements the reference execution engine for Valo.
//! It consumes the validated AST and executes it by traversing the tree.

mod arrays;
pub(crate) mod builtins;
mod calls;
mod control_flow;
mod eval_expr;
mod exec_stmt;
mod ffi;
mod frame;
#[allow(clippy::module_inception)]
mod interpreter;
mod objects;
mod properties;
mod records;
mod values;

pub(crate) use control_flow::ControlFlow;
pub use frame::Frame;
pub use interpreter::{Interpreter, run};
pub(crate) use objects::RuntimeClass;
pub(crate) use records::RuntimeEnum;

#[cfg(test)]
mod tests;
