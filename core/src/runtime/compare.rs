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
                a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
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

pub fn values_identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
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
        value = value.to_ascii_lowercase();
        pattern = pattern.to_ascii_lowercase();
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
    value: Option<char>,
    pattern: &[char],
    start: usize,
) -> Option<(bool, usize)> {
    let mut index = start + 1;
    let negated = pattern.get(index) == Some(&'!');
    if negated {
        index += 1;
    }
    let list_start = index;
    while index < pattern.len() && pattern[index] != ']' {
        index += 1;
    }
    if index >= pattern.len() || index == list_start {
        return None;
    }
    let value = value?;
    let contains = pattern[list_start..index].contains(&value);
    Some((if negated { !contains } else { contains }, index + 1))
}
