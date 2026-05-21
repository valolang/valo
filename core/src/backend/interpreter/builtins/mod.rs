//! Valo Builtins
//!
//! Standard library functions and procedures.
//!
//! TODO: Decouple builtins from the Interpreter and Expr AST.
//! Builtins should ideally take &[Value] and be backend-agnostic.

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
        "VBA"
    } else {
        object_name
    };

    if effective_object_name.eq_ignore_ascii_case("Console")
        || effective_object_name.eq_ignore_ascii_case("Debug")
    {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            let val = interpreter.eval_expr(arg, frame)?;
            let resolved = interpreter.resolve_default_value(val, frame, arg.span)?;
            values.push(resolved);
        }

        if effective_object_name.eq_ignore_ascii_case("Console") {
            return console::exec_console(interpreter, method, &values, span);
        } else {
            return debug::exec_debug(interpreter, method, &values, span);
        }
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

    // Special forms that require lazy evaluation or direct Expr access
    if effective_name.eq_ignore_ascii_case("IIf") {
        expect_arg_count(effective_name, args, 3, span)?;
        let condition = interpreter.eval_expr(&args[0], frame)?.is_truthy();
        let value_expr = if condition { &args[1] } else { &args[2] };
        return Ok(Some(interpreter.eval_expr(value_expr, frame)?));
    }

    if effective_name.eq_ignore_ascii_case("CallByName") {
        return dispatch_callbyname(interpreter, effective_name, args, frame, span);
    }

    if effective_name.eq_ignore_ascii_case("VarPtr") {
        expect_arg_count(effective_name, args, 1, span)?;
        let arg = &args[0];
        let var_name = match &arg.kind {
            crate::ExprKind::Variable(name) => name,
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "VarPtr requires a variable",
                    Some(arg.span),
                ));
            }
        };
        let variable = frame.variable(var_name, arg.span)?;
        let ptr = std::rc::Rc::as_ptr(&variable.cell) as usize;
        return Ok(Some(Value::Ptr(ptr)));
    }

    if effective_name.eq_ignore_ascii_case("StrPtr") {
        expect_arg_count(effective_name, args, 1, span)?;
        let arg = &args[0];
        if let crate::ExprKind::Variable(name) = &arg.kind {
            let variable = frame.variable(name, arg.span)?;
            let value = variable.cell.borrow();
            if let Value::String(s) = &*value {
                return Ok(Some(Value::Ptr(s.as_ptr() as usize)));
            }
            return Ok(Some(Value::Ptr(0)));
        }
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "StrPtr requires a string variable",
            Some(arg.span),
        ));
    }

    if effective_name.eq_ignore_ascii_case("ObjPtr") {
        expect_arg_count(effective_name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        match value {
            Value::Object(obj) => {
                let ptr = std::rc::Rc::as_ptr(&obj) as usize;
                return Ok(Some(Value::Ptr(ptr)));
            }
            Value::Nothing => {
                return Ok(Some(Value::Ptr(0)));
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "ObjPtr requires an object",
                    Some(span),
                ));
            }
        }
    }

    if !is_builtin_function(effective_name) {
        return Ok(None);
    }

    // Normal functions: evaluate all arguments first
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(interpreter.eval_expr(arg, frame)?);
    }

    if effective_name.eq_ignore_ascii_case("IsMissing") {
        if values.len() != 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "IsMissing expects exactly 1 argument",
                Some(span),
            ));
        }
        return Ok(Some(Value::Boolean(matches!(values[0], Value::Missing))));
    }

    if let Some(val) = types::eval_types(effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = arrays::eval_arrays(effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = strings::eval_strings(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }
    if let Some(val) = math::eval_math(interpreter, effective_name, &values, span)? {
        return Ok(Some(val));
    }

    Ok(None)
}

fn is_builtin_function(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "sgn"
            | "int"
            | "randomize"
            | "rnd"
            | "split"
            | "join"
            | "filter"
            | "cstr"
            | "strcomp"
            | "isobject"
            | "isarray"
            | "isnull"
            | "isempty"
            | "iserror"
            | "vartype"
            | "typename"
            | "cbyte"
            | "cint"
            | "clng"
            | "clnglng"
            | "cint64"
            | "csng"
            | "cdbl"
            | "cdec"
            | "ccur"
            | "cdate"
            | "array"
            | "lbound"
            | "ubound"
            | "ismissing"
    )
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
