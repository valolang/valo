//! Valo Runtime
//!
//! The runtime defines the core data model and behavior of the language,
//! including the value system, type definitions, and diagnostics.
//! It is designed to be independent of the execution backend.

mod coerce;
pub mod compare;
mod diagnostic;
pub mod numeric;
pub mod ops;
mod type_name;
mod value;
pub mod ffi_platform;

pub use coerce::coerce_assignment;
pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticLabel, FileId, LabelStyle, RuntimeErrorInfo, Severity,
    SourceMap, SourcePos, Span, terminal_supports_color,
};
pub use type_name::TypeName;
pub use value::{EventBinding, ObjectValue, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayBound {
    pub lower: i64,
    pub upper: i64,
}
