//! Valo Runtime
//!
//! Interpreter values, runtime operations and shared diagnostics.
//! Source types and arithmetic selection belong to the frontend. The runtime
//! executes those decisions while the source interpreter is migrated toward HIR.

pub mod builtins;
pub mod callable;
mod coerce;
pub mod compare;
mod diagnostic;
pub mod ffi_platform;
pub mod naming;
pub mod numeric;
pub mod ops;
pub mod stdlib;
mod type_name;
mod value;
pub mod vba;
pub mod well_known;

pub use crate::frontend::type_model::{TupleElement, TypeName};
pub use coerce::coerce_assignment;
pub use diagnostic::{
    ALL_DIAGNOSTIC_CODES, Diagnostic, DiagnosticCode, DiagnosticLabel, FileId, LabelStyle,
    RuntimeErrorInfo, Severity, SourceMap, SourcePos, Span, terminal_supports_color,
};
pub use naming::{fold, with_folded};
pub use value::{
    ArrayValue, CapturedVariable, CollectionItem, CollectionValue, EventBinding, LambdaValue,
    ObjectValue, RecordValue, Value,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayBound {
    pub lower: i64,
    pub upper: i64,
}
