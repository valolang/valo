use super::super::Frame;
use super::super::Interpreter;
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn eval_arrays(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Array") {
        let mut elements = Vec::new();
        for arg in args {
            elements.push(interpreter.eval_expr(arg, frame)?);
        }
        let len = elements.len() as i64;
        return Ok(Some(Value::Array {
            element_type: crate::runtime::TypeName::Variant,
            elements,
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
        }));
    }

    if name.eq_ignore_ascii_case("LBound") || name.eq_ignore_ascii_case("UBound") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("{} expects one array argument and optional dimension", name),
                Some(span),
            ));
        }
        let dimension = if args.len() == 2 {
            interpreter.eval_integer_expr(&args[1], frame, "Array dimension must be Integer")?
                as usize
        } else {
            1
        };
        let value = interpreter.eval_expr(&args[0], frame)?;
        let bound = if name.eq_ignore_ascii_case("LBound") {
            super::super::arrays::lbound(&value, dimension, span)?
        } else {
            super::super::arrays::ubound(&value, dimension, span)?
        };
        return Ok(Some(Value::Integer(bound)));
    }

    Ok(None)
}
