use super::super::Frame;
use super::super::Interpreter;
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn eval_math(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Sgn") {
        super::expect_arg_count(name, args, 1, span)?;
        let value =
            interpreter.eval_integer_expr(&args[0], frame, "Sgn argument must be Integer")?;
        return Ok(Some(Value::Integer(value.signum())));
    }
    if name.eq_ignore_ascii_case("Int") {
        super::expect_arg_count(name, args, 1, span)?;
        let value =
            interpreter.eval_integer_expr(&args[0], frame, "Int argument must be Integer")?;
        return Ok(Some(Value::Integer(value)));
    }

    Ok(None)
}
