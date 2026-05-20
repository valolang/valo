use super::super::{ControlFlow, Interpreter};
use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_debug(
    interpreter: &mut Interpreter,
    method: &str,
    args: &[Value],
    _span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if method.eq_ignore_ascii_case("Print") {
        let mut parts = Vec::new();
        for value in args {
            parts.push(value.to_output_string());
        }
        interpreter.output.push(parts.join("\t"));
        return Ok(Some(ControlFlow::Continue));
    }

    Ok(None)
}
