use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_console(
    method: &str,
    args: &[Value],
    _span: crate::runtime::Span,
) -> Result<Option<String>, Diagnostic> {
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
        return Ok(Some(parts.join(" ")));
    }

    Ok(None)
}
