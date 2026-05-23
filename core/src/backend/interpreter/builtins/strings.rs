use super::super::Interpreter;
use crate::runtime::numeric::value_to_i64;
use crate::runtime::{ArrayValue, Diagnostic, Value};
use std::rc::Rc;

pub(crate) fn eval_strings(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
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
        let text = args[0].to_output_string();
        let delimiter = if args.len() == 2 {
            args[1].to_output_string()
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
        return Ok(Some(Value::Array(Rc::new(ArrayValue {
            element_type: crate::runtime::TypeName::String,
            elements,
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
        let array_value = &args[0];
        let delimiter = if args.len() == 2 {
            args[1].to_output_string()
        } else {
            " ".to_string()
        };
        let elements = super::super::arrays::array_values(array_value, span)?;
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
        let array_value = &args[0];
        let match_text = args[1].to_output_string();
        let include = if args.len() >= 3 {
            args[2].is_truthy()
        } else {
            true
        };
        let compare = if args.len() == 4 {
            value_to_i64(&args[3]).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Compare mode must be Integer",
                    Some(span),
                )
            })? == 1
        } else {
            interpreter.option_compare == crate::OptionCompare::Text
        };

        let elements = super::super::arrays::array_values(array_value, span)?;
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
        return Ok(Some(Value::Array(Rc::new(ArrayValue {
            element_type: crate::runtime::TypeName::Variant,
            elements: filtered,
            bounds: vec![crate::runtime::ArrayBound {
                lower: 0,
                upper: len - 1,
            }],
            allocated: true,
            dynamic: true,
        }))));
    }

    if name.eq_ignore_ascii_case("CStr") {
        if args.len() != 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "CStr expects exactly 1 argument",
                Some(span),
            ));
        }
        return Ok(Some(Value::String(args[0].to_output_string())));
    }

    if name.eq_ignore_ascii_case("Len") {
        if args.len() != 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Len expects exactly 1 argument",
                Some(span),
            ));
        }
        let s = args[0].to_output_string();
        return Ok(Some(Value::Int64(s.chars().count() as i64)));
    }

    if name.eq_ignore_ascii_case("LenB") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::Int64(args[0].to_output_string().len() as i64)));
    }

    if name.eq_ignore_ascii_case("Left") {
        expect_arg_range(name, args, 2, 2, span)?;
        let text = args[0].to_output_string();
        let count = non_negative_len(name, &args[1], span)?;
        return Ok(Some(Value::String(text.chars().take(count).collect())));
    }

    if name.eq_ignore_ascii_case("Right") {
        expect_arg_range(name, args, 2, 2, span)?;
        let text = args[0].to_output_string();
        let count = non_negative_len(name, &args[1], span)?;
        let len = text.chars().count();
        return Ok(Some(Value::String(
            text.chars().skip(len.saturating_sub(count)).collect(),
        )));
    }

    if name.eq_ignore_ascii_case("Mid") {
        expect_arg_range(name, args, 2, 3, span)?;
        let text = args[0].to_output_string();
        let start = one_based_start(name, &args[1], span)?;
        let chars = text.chars().skip(start.saturating_sub(1));
        let result: String = if let Some(length) = args.get(2) {
            chars.take(non_negative_len(name, length, span)?).collect()
        } else {
            chars.collect()
        };
        return Ok(Some(Value::String(result)));
    }

    if name.eq_ignore_ascii_case("Trim") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::String(
            args[0].to_output_string().trim().to_string(),
        )));
    }

    if name.eq_ignore_ascii_case("LTrim") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::String(
            args[0].to_output_string().trim_start().to_string(),
        )));
    }

    if name.eq_ignore_ascii_case("RTrim") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::String(
            args[0].to_output_string().trim_end().to_string(),
        )));
    }

    if name.eq_ignore_ascii_case("UCase") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::String(
            args[0].to_output_string().to_uppercase(),
        )));
    }

    if name.eq_ignore_ascii_case("LCase") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::String(
            args[0].to_output_string().to_lowercase(),
        )));
    }

    if name.eq_ignore_ascii_case("Replace") {
        expect_arg_range(name, args, 3, 6, span)?;
        let expression = args[0].to_output_string();
        let find = args[1].to_output_string();
        let replacement = args[2].to_output_string();
        let start = if let Some(start) = args.get(3) {
            one_based_start(name, start, span)?
        } else {
            1
        };
        let count = if let Some(count) = args.get(4) {
            value_to_i64(count).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Replace count must be Integer",
                    Some(span),
                )
            })?
        } else {
            -1
        };
        let compare_text = args
            .get(5)
            .map(|value| compare_is_text(value, span))
            .transpose()?
            .unwrap_or(interpreter.option_compare == crate::OptionCompare::Text);
        let prefix: String = expression.chars().take(start.saturating_sub(1)).collect();
        let tail: String = expression.chars().skip(start.saturating_sub(1)).collect();
        let replaced = replace_limited(&tail, &find, &replacement, count, compare_text);
        return Ok(Some(Value::String(format!("{prefix}{replaced}"))));
    }

    if name.eq_ignore_ascii_case("InStr") {
        expect_arg_range(name, args, 2, 4, span)?;
        let (start, text, find, compare_arg) = match args.len() {
            2 => (
                1,
                args[0].to_output_string(),
                args[1].to_output_string(),
                None,
            ),
            3 => (
                one_based_start(name, &args[0], span)?,
                args[1].to_output_string(),
                args[2].to_output_string(),
                None,
            ),
            _ => (
                one_based_start(name, &args[0], span)?,
                args[1].to_output_string(),
                args[2].to_output_string(),
                Some(&args[3]),
            ),
        };
        let compare_text = compare_arg
            .map(|value| compare_is_text(value, span))
            .transpose()?
            .unwrap_or(interpreter.option_compare == crate::OptionCompare::Text);
        return Ok(Some(Value::Int64(
            instr(&text, &find, start, compare_text).unwrap_or(0),
        )));
    }

    if name.eq_ignore_ascii_case("InStrRev") {
        expect_arg_range(name, args, 2, 4, span)?;
        let text = args[0].to_output_string();
        let find = args[1].to_output_string();
        let start = if let Some(start) = args.get(2) {
            value_to_i64(start).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "InStrRev start must be Integer",
                    Some(span),
                )
            })?
        } else {
            -1
        };
        let compare_text = args
            .get(3)
            .map(|value| compare_is_text(value, span))
            .transpose()?
            .unwrap_or(interpreter.option_compare == crate::OptionCompare::Text);
        return Ok(Some(Value::Int64(instr_rev(
            &text,
            &find,
            start,
            compare_text,
        ))));
    }

    if name.eq_ignore_ascii_case("Space") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::String(
            " ".repeat(non_negative_len(name, &args[0], span)?),
        )));
    }

    if name.eq_ignore_ascii_case("String") {
        expect_arg_range(name, args, 2, 2, span)?;
        let count = non_negative_len(name, &args[0], span)?;
        let ch = string_char(&args[1], span)?;
        return Ok(Some(Value::String(ch.to_string().repeat(count))));
    }

    if name.eq_ignore_ascii_case("Chr") || name.eq_ignore_ascii_case("ChrW") {
        expect_arg_range(name, args, 1, 1, span)?;
        let code = value_to_i64(&args[0]).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Chr code must be Integer",
                Some(span),
            )
        })?;
        let Some(ch) = char::from_u32(code as u32) else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Chr code is outside the supported Unicode scalar range",
                Some(span),
            ));
        };
        return Ok(Some(Value::String(ch.to_string())));
    }

    if name.eq_ignore_ascii_case("Asc") || name.eq_ignore_ascii_case("AscW") {
        expect_arg_range(name, args, 1, 1, span)?;
        let text = args[0].to_output_string();
        let Some(ch) = text.chars().next() else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Asc requires a non-empty string",
                Some(span),
            ));
        };
        return Ok(Some(Value::Int64(ch as i64)));
    }

    if name.eq_ignore_ascii_case("Val") {
        expect_arg_range(name, args, 1, 1, span)?;
        return Ok(Some(Value::Double(val_number(&args[0].to_output_string()))));
    }

    if name.eq_ignore_ascii_case("Str") {
        expect_arg_range(name, args, 1, 1, span)?;
        let text = args[0].to_output_string();
        let prefix = if text.starts_with('-') { "" } else { " " };
        return Ok(Some(Value::String(format!("{prefix}{text}"))));
    }

    if name.eq_ignore_ascii_case("Hex") || name.eq_ignore_ascii_case("Oct") {
        expect_arg_range(name, args, 1, 1, span)?;
        let value = value_to_i64(&args[0]).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                format!("{name} argument must be Integer"),
                Some(span),
            )
        })?;
        let text = if name.eq_ignore_ascii_case("Hex") {
            format!("{value:X}")
        } else {
            format!("{value:o}")
        };
        return Ok(Some(Value::String(text)));
    }

    if name.eq_ignore_ascii_case("StrComp") {
        if args.len() < 2 || args.len() > 3 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "StrComp expects two strings and optional compare mode",
                Some(span),
            ));
        }
        let left = args[0].to_output_string();
        let right = args[1].to_output_string();
        let text_compare = if args.len() == 3 {
            value_to_i64(&args[2]).ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Compare mode must be Integer",
                    Some(span),
                )
            })? == 1
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

fn expect_arg_range(
    name: &str,
    args: &[Value],
    min: usize,
    max: usize,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if (min..=max).contains(&args.len()) {
        Ok(())
    } else if min == max {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects exactly {min} argument(s)"),
            Some(span),
        ))
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects {min} to {max} arguments"),
            Some(span),
        ))
    }
}

fn non_negative_len(
    name: &str,
    value: &Value,
    span: crate::runtime::Span,
) -> Result<usize, Diagnostic> {
    let count = value_to_i64(value).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} length must be Integer"),
            Some(span),
        )
    })?;
    if count < 0 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} length cannot be negative"),
            Some(span),
        ));
    }
    Ok(count as usize)
}

fn one_based_start(
    name: &str,
    value: &Value,
    span: crate::runtime::Span,
) -> Result<usize, Diagnostic> {
    let start = value_to_i64(value).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} start must be Integer"),
            Some(span),
        )
    })?;
    if start < 1 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!("{name} start must be at least 1"),
            Some(span),
        ));
    }
    Ok(start as usize)
}

fn compare_is_text(value: &Value, span: crate::runtime::Span) -> Result<bool, Diagnostic> {
    match value_to_i64(value) {
        Some(-1) => Ok(false),
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        Some(2) => Ok(true),
        Some(_) | None => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Compare mode must be vbUseCompareOption, vbBinaryCompare, vbTextCompare, or vbDatabaseCompare",
            Some(span),
        )),
    }
}

fn replace_limited(
    text: &str,
    find: &str,
    replacement: &str,
    count: i64,
    compare_text: bool,
) -> String {
    if find.is_empty() || count == 0 {
        return text.to_string();
    }
    let mut result = String::new();
    let mut rest = text;
    let mut replaced = 0;
    loop {
        if count >= 0 && replaced >= count {
            result.push_str(rest);
            break;
        }
        let haystack = if compare_text {
            rest.to_lowercase()
        } else {
            rest.to_string()
        };
        let needle = if compare_text {
            find.to_lowercase()
        } else {
            find.to_string()
        };
        let Some(pos) = haystack.find(&needle) else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..pos]);
        result.push_str(replacement);
        rest = &rest[pos + find.len()..];
        replaced += 1;
    }
    result
}

fn instr(text: &str, find: &str, start: usize, compare_text: bool) -> Option<i64> {
    if find.is_empty() {
        return Some(start as i64);
    }
    let prefix_len = text
        .char_indices()
        .nth(start.saturating_sub(1))
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    let tail = &text[prefix_len..];
    let haystack = if compare_text {
        tail.to_lowercase()
    } else {
        tail.to_string()
    };
    let needle = if compare_text {
        find.to_lowercase()
    } else {
        find.to_string()
    };
    haystack.find(&needle).map(|byte_pos| {
        let char_offset = tail[..byte_pos].chars().count();
        (start + char_offset) as i64
    })
}

fn instr_rev(text: &str, find: &str, start: i64, compare_text: bool) -> i64 {
    let end_chars = if start < 1 {
        text.chars().count()
    } else {
        start as usize
    };
    let haystack: String = text.chars().take(end_chars).collect();
    if find.is_empty() {
        return end_chars as i64;
    }
    let searchable = if compare_text {
        haystack.to_lowercase()
    } else {
        haystack.clone()
    };
    let needle = if compare_text {
        find.to_lowercase()
    } else {
        find.to_string()
    };
    searchable
        .rfind(&needle)
        .map(|byte_pos| haystack[..byte_pos].chars().count() as i64 + 1)
        .unwrap_or(0)
}

fn string_char(value: &Value, span: crate::runtime::Span) -> Result<char, Diagnostic> {
    if let Some(code) = value_to_i64(value)
        && let Some(ch) = char::from_u32(code as u32)
    {
        return Ok(ch);
    }
    let text = value.to_output_string();
    text.chars().next().ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "String character argument cannot be empty",
            Some(span),
        )
    })
}

fn val_number(text: &str) -> f64 {
    let trimmed = text.trim_start();
    let mut end = 0;
    let mut saw_digit = false;
    for (idx, ch) in trimmed.char_indices() {
        let valid = ch.is_ascii_digit()
            || matches!(ch, '+' | '-' | '.')
            || ((ch == 'e' || ch == 'E') && saw_digit);
        if !valid {
            break;
        }
        if ch.is_ascii_digit() {
            saw_digit = true;
        }
        end = idx + ch.len_utf8();
    }
    if !saw_digit {
        return 0.0;
    }
    trimmed[..end].parse::<f64>().unwrap_or(0.0)
}
