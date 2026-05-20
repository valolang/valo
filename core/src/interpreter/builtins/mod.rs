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
    // Handle VBA namespace fallback: VBA.MsgBox(...) -> MsgBox(...)
    let effective_object_name = if object_name.eq_ignore_ascii_case("VBA") {
        // If it's VBA.Something, we might want to treat Something as a global sub call
        // if it's one of our builtins.
        // For now, let's just handle it by checking if it matches our builtin sub targets.
        "VBA"
    } else {
        object_name
    };

    if effective_object_name.eq_ignore_ascii_case("Console") {
        return console::exec_console(interpreter, method, args, frame, span);
    }
    if effective_object_name.eq_ignore_ascii_case("Debug") {
        return debug::exec_debug(interpreter, method, args, frame, span);
    }
    if effective_object_name.eq_ignore_ascii_case("Err") {
        return err::exec_err(interpreter, method, args, frame, span);
    }

    if object_name.eq_ignore_ascii_case("VBA") {
        // VBA.Randomize 123
        if let Some(val) = dispatch_function(interpreter, method, args, frame, span)? {
            // If it returns a value but was called as a stmt, we just ignore the value
            // (or maybe check if it's a valid stmt builtin)
            if matches!(val, Value::Empty) || method.eq_ignore_ascii_case("Randomize") {
                return Ok(Some(ControlFlow::Continue));
            }
        }
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
    // Handle VBA namespace fallback: VBA.Join(...) -> Join(...)
    let effective_name = if let Some(stripped) = name.strip_prefix("VBA.") {
        stripped
    } else {
        name
    };

    if effective_name.eq_ignore_ascii_case("IsMissing") {
        expect_arg_count(effective_name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Boolean(matches!(value, Value::Missing))));
    }

    if let Some(val) = types::eval_types(interpreter, effective_name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = arrays::eval_arrays(interpreter, effective_name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = strings::eval_strings(interpreter, effective_name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = math::eval_math(interpreter, effective_name, args, frame, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = dispatch_callbyname(interpreter, effective_name, args, frame, span)? {
        return Ok(Some(val));
    }

    Ok(None)
}

fn dispatch_callbyname(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("CallByName") {
        if args.len() < 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CallByName expects at least 3 arguments",
                Some(span),
            ));
        }
        let obj = interpreter.eval_expr(&args[0], frame)?;
        let member = interpreter.eval_expr(&args[1], frame)?.to_output_string();
        let call_type =
            interpreter.eval_integer_expr(&args[2], frame, "Call type must be Integer")?;

        let remaining_args = &args[3..];

        match call_type {
            1 => {
                // VbMethod
                if let Err(err) =
                    interpreter.call_method_sub(obj.clone(), &member, remaining_args, frame, span)
                {
                    if matches!(err.code, crate::runtime::DiagnosticCode("V1400")) {
                        // Not found as Sub, try Function
                        return Ok(Some(interpreter.call_method_function(
                            obj,
                            &member,
                            remaining_args,
                            frame,
                            span,
                        )?));
                    }
                    return Err(err);
                }
                return Ok(Some(Value::Empty));
            }
            2 => {
                // VbGet
                return Ok(Some(interpreter.read_member(&obj, &member, frame, span)?));
            }
            4 | 8 => {
                // VbLet (4) or VbSet (8)
                if remaining_args.len() != 1 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "CallByName for Let/Set expects exactly one value argument",
                        Some(span),
                    ));
                }
                let value = interpreter.eval_expr(&remaining_args[0], frame)?;
                interpreter.assign_member_to_value(obj, &member, value, span)?;
                return Ok(Some(Value::Empty));
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    format!("Invalid CallByName call type: {}", call_type),
                    Some(span),
                ));
            }
        }
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
