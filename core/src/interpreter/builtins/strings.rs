use super::super::Frame;
use super::super::Interpreter;
use crate::Expr;
use crate::runtime::{Diagnostic, Value};

pub(crate) fn eval_strings(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("Split") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Split expects 1 to 2 arguments",
                Some(span),
            ));
        }
        let text = interpreter.eval_expr(&args[0], frame)?.to_output_string();
        let delimiter = if args.len() == 2 {
            interpreter.eval_expr(&args[1], frame)?.to_output_string()
        } else {
            " ".to_string()
        };
        let elements: Vec<Value> = if delimiter.is_empty() {
            vec![Value::String(text)]
        } else {
            text.split(&delimiter)
                .map(|s| Value::String(s.to_string()))
                .collect()
        };
        let len = elements.len() as i64;
        return Ok(Some(Value::Array {
            element_type: crate::runtime::TypeName::String,
            elements,
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
        }));
    }

    if name.eq_ignore_ascii_case("Join") {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Join expects 1 to 2 arguments",
                Some(span),
            ));
        }
        let array_value = interpreter.eval_expr(&args[0], frame)?;
        let delimiter = if args.len() == 2 {
            interpreter.eval_expr(&args[1], frame)?.to_output_string()
        } else {
            " ".to_string()
        };
        let elements = super::super::arrays::array_values(&array_value, args[0].span)?;
        let strings: Vec<String> = elements.iter().map(|v| v.to_output_string()).collect();
        return Ok(Some(Value::String(strings.join(&delimiter))));
    }

    if name.eq_ignore_ascii_case("Filter") {
        if args.len() < 2 || args.len() > 4 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Filter expects 2 to 4 arguments",
                Some(span),
            ));
        }
        let array_value = interpreter.eval_expr(&args[0], frame)?;
        let match_text = interpreter.eval_expr(&args[1], frame)?.to_output_string();
        let include = if args.len() >= 3 {
            interpreter.eval_expr(&args[2], frame)?.is_truthy()
        } else {
            true
        };
        let compare = if args.len() == 4 {
            interpreter.eval_integer_expr(&args[3], frame, "Compare mode must be Integer")? == 1
        } else {
            interpreter.option_compare == crate::OptionCompare::Text
        };

        let elements = super::super::arrays::array_values(&array_value, args[0].span)?;
        let mut filtered = Vec::new();
        for val in elements {
            let s = val.to_output_string();
            let contains = if compare {
                s.to_ascii_lowercase()
                    .contains(&match_text.to_ascii_lowercase())
            } else {
                s.contains(&match_text)
            };
            if contains == include {
                filtered.push(val);
            }
        }
        let len = filtered.len() as i64;
        return Ok(Some(Value::Array {
            element_type: crate::runtime::TypeName::Variant,
            elements: filtered,
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
        }));
    }

    if name.eq_ignore_ascii_case("CStr") {
        super::expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::String(value.to_output_string())));
    }

    if name.eq_ignore_ascii_case("StrComp") {
        if args.len() < 2 || args.len() > 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "StrComp expects two strings and optional compare mode",
                Some(span),
            ));
        }
        let left = interpreter.eval_expr(&args[0], frame)?.to_output_string();
        let right = interpreter.eval_expr(&args[1], frame)?.to_output_string();
        let text_compare = if args.len() == 3 {
            interpreter.eval_integer_expr(&args[2], frame, "Compare mode must be Integer")? == 1
        } else {
            interpreter.option_compare == crate::OptionCompare::Text
        };
        let (left, right) = if text_compare {
            (left.to_ascii_lowercase(), right.to_ascii_lowercase())
        } else {
            (left, right)
        };
        let result = match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        return Ok(Some(Value::Int64(result)));
    }

    Ok(None)
}
