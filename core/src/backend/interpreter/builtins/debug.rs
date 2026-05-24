use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_debug(
    method: &str,
    args: &[Value],
    _span: crate::runtime::Span,
) -> Result<Option<String>, Diagnostic> {
    if method.eq_ignore_ascii_case("Print") {
        let mut parts = Vec::new();
        for value in args {
            parts.push(value.to_output_string());
        }
        return Ok(Some(parts.join("\t")));
    }

    Ok(None)
}
