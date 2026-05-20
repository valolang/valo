use super::super::{ControlFlow, Frame, Interpreter};
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn exec_err(
    interpreter: &mut Interpreter,
    method: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if method.eq_ignore_ascii_case("Clear") && args.is_empty() {
        interpreter.clear_err();
        return Ok(Some(ControlFlow::Continue));
    }
    if method.eq_ignore_ascii_case("Raise") {
        return Err(interpreter.err_raise(args, frame, span)?);
    }

    Ok(None)
}

pub(crate) fn eval_err(
    interpreter: &Interpreter,
    field: &str,
    _span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if field.eq_ignore_ascii_case("Number") {
        return Ok(Some(Value::Integer(interpreter.err_number)));
    }
    if field.eq_ignore_ascii_case("Description") {
        return Ok(Some(Value::String(interpreter.err_description.clone())));
    }
    if field.eq_ignore_ascii_case("Source") {
        return Ok(Some(Value::String(interpreter.err_source.clone())));
    }
    if field.eq_ignore_ascii_case("HelpFile") {
        return Ok(Some(Value::String(interpreter.err_help_file.clone())));
    }
    if field.eq_ignore_ascii_case("HelpContext") {
        return Ok(Some(Value::Integer(interpreter.err_help_context)));
    }

    Ok(None)
}
