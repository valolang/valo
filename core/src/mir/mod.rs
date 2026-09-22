//! Backend-neutral, typed control-flow IR for the verified-HIR subset.
pub mod debug;
pub mod ir;
pub mod lower;
pub mod verify;

pub use lower::{LowerError, lower_body, lower_module};
