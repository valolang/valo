use crate::runtime::numeric::value_to_f64;
use crate::runtime::{Diagnostic, Span, Value};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeOptionCompare {
    Binary,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCompareOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

pub fn compare_case_values(
    left: Value,
    op: RuntimeCompareOp,
    right: Value,
    compare: RuntimeOptionCompare,
    span: Span,
) -> Result<Value, Diagnostic> {
    match op {
        RuntimeCompareOp::Equal => Ok(Value::Boolean(values_equal(&left, &right, compare))),
        RuntimeCompareOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right, compare))),
        RuntimeCompareOp::Less => compare_values(left, right, compare, span, |ord| ord.is_lt()),
        RuntimeCompareOp::Greater => compare_values(left, right, compare, span, |ord| ord.is_gt()),
        RuntimeCompareOp::LessEqual => {
            compare_values(left, right, compare, span, |ord| ord.is_le())
        }
        RuntimeCompareOp::GreaterEqual => {
            compare_values(left, right, compare, span, |ord| ord.is_ge())
        }
    }
}

pub fn compare_values(
    left: Value,
    right: Value,
    compare: RuntimeOptionCompare,
    span: Span,
    predicate: impl FnOnce(std::cmp::Ordering) -> bool,
) -> Result<Value, Diagnostic> {
    let ordering = match (left, right) {
        (Value::String(a), Value::String(b)) => {
            if compare == RuntimeOptionCompare::Text {
                a.to_lowercase().cmp(&b.to_lowercase())
            } else {
                a.cmp(&b)
            }
        }
        (l, r) => {
            if let (Some(a), Some(b)) = (value_to_f64(&l), value_to_f64(&r)) {
                a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Comparison requires matching numeric or String operands",
                    Some(span),
                ));
            }
        }
    };

    Ok(Value::Boolean(predicate(ordering)))
}

pub fn values_equal(left: &Value, right: &Value, compare: RuntimeOptionCompare) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => {
            if compare == RuntimeOptionCompare::Text {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        }
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Empty, Value::Empty) => true,
        (Value::Null, Value::Null) => true,
        (l, r) => {
            if let (Some(a), Some(b)) = (value_to_f64(l), value_to_f64(r)) {
                a == b
            } else {
                false
            }
        }
    }
}

fn unwrap_nullable(v: &Value) -> &Value {
    if let Value::Nullable(inner) = v {
        inner.as_ref()
    } else {
        v
    }
}

pub fn values_identical(left: &Value, right: &Value) -> bool {
    match (unwrap_nullable(left), unwrap_nullable(right)) {
        (Value::Nothing, Value::Nothing) => true,
        (Value::Object(left), Value::Object(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

pub fn like_values(
    left: Value,
    right: Value,
    compare: RuntimeOptionCompare,
    span: Span,
) -> Result<Value, Diagnostic> {
    let (Value::String(mut value), Value::String(mut pattern)) = (left, right) else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Like requires String operands",
            Some(span),
        ));
    };
    if compare == RuntimeOptionCompare::Text {
        value = value.to_lowercase();
        pattern = pattern.to_lowercase();
    }
    Ok(Value::Boolean(like_match(
        &value.chars().collect::<Vec<_>>(),
        &pattern.chars().collect::<Vec<_>>(),
    )))
}

pub fn like_match(value: &[char], pattern: &[char]) -> bool {
    fn inner(value: &[char], pattern: &[char], vi: usize, pi: usize) -> bool {
        if pi == pattern.len() {
            return vi == value.len();
        }
        match pattern[pi] {
            '*' => {
                for next in vi..=value.len() {
                    if inner(value, pattern, next, pi + 1) {
                        return true;
                    }
                }
                false
            }
            '?' => vi < value.len() && inner(value, pattern, vi + 1, pi + 1),
            '#' => {
                vi < value.len()
                    && value[vi].is_ascii_digit()
                    && inner(value, pattern, vi + 1, pi + 1)
            }
            '[' => {
                let Some((matches, next_pi)) = match_char_list(value.get(vi).copied(), pattern, pi)
                else {
                    return vi < value.len()
                        && value[vi] == '['
                        && inner(value, pattern, vi + 1, pi + 1);
                };
                matches && inner(value, pattern, vi + 1, next_pi)
            }
            literal => {
                vi < value.len() && value[vi] == literal && inner(value, pattern, vi + 1, pi + 1)
            }
        }
    }
    inner(value, pattern, 0, 0)
}

pub fn match_char_list(
    value_char: Option<char>,
    pattern: &[char],
    start: usize,
) -> Option<(bool, usize)> {
    let value = value_char?;
    let mut pi = start + 1;
    let negated = pattern.get(pi) == Some(&'!');
    if negated {
        pi += 1;
    }

    let mut found_match = false;

    // Special case: ']' at the very beginning of the list matches literally
    if pattern.get(pi) == Some(&']') {
        if value == ']' {
            found_match = true;
        }
        pi += 1;
    }

    while pi < pattern.len() && pattern[pi] != ']' {
        // Check for range x-y
        // A range is valid if:
        // 1. We have at least 3 chars left (x-y)
        // 2. The next char is '-'
        // 3. The char after '-' is not ']'
        if pi + 2 < pattern.len() && pattern[pi + 1] == '-' && pattern[pi + 2] != ']' {
            let start_range = pattern[pi];
            let end_range = pattern[pi + 2];
            if value >= start_range && value <= end_range {
                found_match = true;
            }
            pi += 3;
        } else {
            if value == pattern[pi] {
                found_match = true;
            }
            pi += 1;
        }
    }

    if pi >= pattern.len() || pattern[pi] != ']' {
        return None;
    }

    Some((if negated { !found_match } else { found_match }, pi + 1))
}
