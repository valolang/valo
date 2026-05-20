use super::super::{ControlFlow, Frame, Interpreter};
use crate::Expr;
use crate::runtime::Diagnostic;

pub(crate) fn exec_debug(
    interpreter: &mut Interpreter,
    method: &str,
    args: &[Expr],
    frame: &mut Frame,
    _span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if method.eq_ignore_ascii_case("Print") {
        let mut parts = Vec::new();
        for arg in args {
            let value = interpreter.eval_expr(arg, frame)?;
            parts.push(
                interpreter
                    .resolve_default_value(value, frame, arg.span)?
                    .to_output_string(),
            );
        }
        interpreter.output.push(parts.join("\t"));
        return Ok(Some(ControlFlow::Continue));
    }

    Ok(None)
}
