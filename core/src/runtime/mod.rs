mod diagnostic;
mod type_name;
mod value;

pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticLabel, LabelStyle, RuntimeErrorInfo, Severity, SourcePos,
    Span,
};
pub use type_name::TypeName;
pub use value::{EventBinding, ObjectValue, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayBound {
    pub lower: i64,
    pub upper: i64,
}
