use super::super::{ControlFlow, Frame, Interpreter};
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_console(
    interpreter: &mut Interpreter,
    method: &str,
    args: &[Expr],
    frame: &mut Frame,
    _span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if method.eq_ignore_ascii_case("WriteLine") {
        let mut parts = Vec::new();
        for arg in args {
            let value = interpreter.eval_expr(arg, frame)?;
            if matches!(value, Value::Missing) {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Missing optional argument cannot be used as a value",
                    Some(arg.span),
                ));
            }
            parts.push(
                interpreter
                    .resolve_default_value(value, frame, arg.span)?
                    .to_output_string(),
            );
        }
        interpreter.output.push(parts.join(" "));
        return Ok(Some(ControlFlow::Continue));
    }

    Ok(None)
}
