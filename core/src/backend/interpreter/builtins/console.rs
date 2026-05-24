use super::super::{ControlFlow, Interpreter};
use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_console(
    interpreter: &mut Interpreter,
    method: &str,
    args: &[Value],
    _span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if method.eq_ignore_ascii_case("WriteLine") {
        let mut parts = Vec::new();
        for value in args {
            if matches!(value, Value::Missing) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Optional argument was omitted here and cannot be printed as a value",
                    None,
                ));
            }
            parts.push(value.to_output_string());
        }
        interpreter.output.push(parts.join(" "));
        return Ok(Some(ControlFlow::Continue));
    }

    Ok(None)
}
