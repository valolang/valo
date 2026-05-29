use crate::runtime::numeric::value_to_i64;
use crate::runtime::{ArrayValue, Diagnostic, Value};
use std::rc::Rc;

pub(crate) fn eval_arrays(
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Array") {
        let len = args.len() as i64;
        return Ok(Some(Value::Array(Rc::new(ArrayValue {
            element_type: crate::runtime::TypeName::Variant,
            elements: args.to_vec(),
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
            dynamic: true,
        }))));
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
            value_to_i64(&args[1]).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Array dimension must be Integer",
                    Some(span),
                )
            })? as usize
        } else {
            1
        };
        let value = &args[0];
        let bound = if name.eq_ignore_ascii_case("LBound") {
            super::super::arrays::lbound(value, dimension, span)?
        } else {
            super::super::arrays::ubound(value, dimension, span)?
        };
        return Ok(Some(Value::Int64(bound)));
    }

    if name.eq_ignore_ascii_case("Split") {
        if args.is_empty() || args.len() > 4 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Split expects 1 to 4 arguments",
                Some(span),
            ));
        }
        let expression = args[0].to_output_string();
        let delimiter = if args.len() >= 2 && !matches!(args[1], Value::Missing) {
            args[1].to_output_string()
        } else {
            " ".to_string()
        };

        let parts: Vec<Value> = if delimiter.is_empty() {
            vec![Value::String(expression)]
        } else {
            expression
                .split(&delimiter)
                .map(|s| Value::String(s.to_string()))
                .collect()
        };

        let len = parts.len() as i64;
        return Ok(Some(Value::Array(Rc::new(ArrayValue {
            element_type: crate::runtime::TypeName::String,
            elements: parts,
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
            dynamic: true,
        }))));
    }

    if name.eq_ignore_ascii_case("Join") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Join expects 1 to 2 arguments",
                Some(span),
            ));
        }
        let Value::Array(arr) = &args[0] else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Join requires an array",
                Some(span),
            ));
        };
        let delimiter = if args.len() == 2 && !matches!(args[1], Value::Missing) {
            args[1].to_output_string()
        } else {
            " ".to_string()
        };
        let strings: Vec<String> = arr.elements.iter().map(|v| v.to_output_string()).collect();
        return Ok(Some(Value::String(strings.join(&delimiter))));
    }

    Ok(None)
}
