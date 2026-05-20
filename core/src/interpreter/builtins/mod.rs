use super::{ControlFlow, Frame, Interpreter};
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn dispatch_stmt(
    interpreter: &mut Interpreter,
    object_name: &str,
    method: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<ControlFlow>, Diagnostic> {
    if object_name.eq_ignore_ascii_case("Console") {
        return console::exec_console(interpreter, method, args, frame, span);
    }
    if object_name.eq_ignore_ascii_case("Debug") {
        return debug::exec_debug(interpreter, method, args, frame, span);
    }
    if object_name.eq_ignore_ascii_case("Err") {
        return err::exec_err(interpreter, method, args, frame, span);
    }

    Ok(None)
}

pub(crate) fn dispatch_function(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("IsMissing") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Boolean(matches!(value, Value::Missing))));
    }

    if let Some(val) = types::eval_types(interpreter, name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = arrays::eval_arrays(interpreter, name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = strings::eval_strings(interpreter, name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = math::eval_math(interpreter, name, args, frame, span)? {
        return Ok(Some(val));
    }

    Ok(None)
}

pub(crate) mod arrays;
pub(crate) mod console;
pub(crate) mod debug;
pub(crate) mod err;
pub(crate) mod math;
pub(crate) mod strings;
pub(crate) mod types;

pub(crate) fn expect_arg_count(
    name: &str,
    args: &[Expr],
    expected: usize,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects exactly {expected} argument(s)"),
            Some(span),
        ))
    }
}
