use std::collections::HashMap;
use std::rc::Rc;

use crate::BinaryOp;
use crate::OptionCompare;
use crate::runtime::{Diagnostic, Span, TypeName, Value};

use super::records::{RuntimeEnum, RuntimeType};

pub(crate) fn eval_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
    compare: OptionCompare,
    span: Span,
) -> Result<Value, Diagnostic> {
    match op {
        BinaryOp::Add => integer_binary(left, right, span, |a, b| a + b),
        BinaryOp::Subtract => integer_binary(left, right, span, |a, b| a - b),
        BinaryOp::Multiply => integer_binary(left, right, span, |a, b| a * b),
        BinaryOp::Divide => {
            let (a, b) = expect_integers(left, right, span)?;
            if b == 0 {
                return Err(Diagnostic::new("Division by zero", Some(span)));
            }
            Ok(Value::Integer(a / b))
        }
        BinaryOp::Modulo => {
            let (a, b) = expect_integers(left, right, span)?;
            if b == 0 {
                return Err(Diagnostic::new("Modulo by zero", Some(span)));
            }
            Ok(Value::Integer(a % b))
        }
        BinaryOp::Concat => Ok(Value::String(format!(
            "{}{}",
            left.to_output_string(),
            right.to_output_string()
        ))),
        BinaryOp::LogicalAnd => logical_or_bitwise(left, right, span, |a, b| a && b, |a, b| a & b),
        BinaryOp::LogicalOr => logical_or_bitwise(left, right, span, |a, b| a || b, |a, b| a | b),
        BinaryOp::Equal => Ok(Value::Boolean(values_equal(&left, &right, compare))),
        BinaryOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right, compare))),
        BinaryOp::Is => Ok(Value::Boolean(values_identical(&left, &right))),
        BinaryOp::Like => like_values(left, right, compare, span),
        BinaryOp::Less => compare_values(left, right, compare, span, |ord| ord.is_lt()),
        BinaryOp::Greater => compare_values(left, right, compare, span, |ord| ord.is_gt()),
        BinaryOp::LessEqual => compare_values(left, right, compare, span, |ord| ord.is_le()),
        BinaryOp::GreaterEqual => compare_values(left, right, compare, span, |ord| ord.is_ge()),
    }
}

fn like_values(
    left: Value,
    right: Value,
    compare: OptionCompare,
    span: Span,
) -> Result<Value, Diagnostic> {
    let (Value::String(mut value), Value::String(mut pattern)) = (left, right) else {
        return Err(Diagnostic::new("Like requires String operands", Some(span)));
    };
    if compare == OptionCompare::Text {
        value = value.to_ascii_lowercase();
        pattern = pattern.to_ascii_lowercase();
    }
    Ok(Value::Boolean(like_match(
        &value.chars().collect::<Vec<_>>(),
        &pattern.chars().collect::<Vec<_>>(),
    )))
}

fn like_match(value: &[char], pattern: &[char]) -> bool {
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

fn match_char_list(value: Option<char>, pattern: &[char], start: usize) -> Option<(bool, usize)> {
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

pub(crate) fn compare_case_values(
    left: Value,
    op: crate::CaseCompareOp,
    right: Value,
    compare: OptionCompare,
    span: Span,
) -> Result<Value, Diagnostic> {
    match op {
        crate::CaseCompareOp::Equal => Ok(Value::Boolean(values_equal(&left, &right, compare))),
        crate::CaseCompareOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right, compare))),
        crate::CaseCompareOp::Less => compare_values(left, right, compare, span, |ord| ord.is_lt()),
        crate::CaseCompareOp::Greater => {
            compare_values(left, right, compare, span, |ord| ord.is_gt())
        }
        crate::CaseCompareOp::LessEqual => {
            compare_values(left, right, compare, span, |ord| ord.is_le())
        }
        crate::CaseCompareOp::GreaterEqual => {
            compare_values(left, right, compare, span, |ord| ord.is_ge())
        }
    }
}

fn integer_binary(
    left: Value,
    right: Value,
    span: Span,
    op: impl FnOnce(i64, i64) -> i64,
) -> Result<Value, Diagnostic> {
    let (a, b) = expect_integers(left, right, span)?;
    Ok(Value::Integer(op(a, b)))
}

fn expect_integers(left: Value, right: Value, span: Span) -> Result<(i64, i64), Diagnostic> {
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Ok((a, b)),
        _ => Err(Diagnostic::new(
            "Arithmetic operators require Integer operands",
            Some(span),
        )),
    }
}

fn logical_or_bitwise(
    left: Value,
    right: Value,
    span: Span,
    bool_op: impl FnOnce(bool, bool) -> bool,
    int_op: impl FnOnce(i64, i64) -> i64,
) -> Result<Value, Diagnostic> {
    match (left, right) {
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(bool_op(a, b))),
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(int_op(a, b))),
        _ => Err(Diagnostic::new(
            "Logical operators require Boolean or Integer operands",
            Some(span),
        )),
    }
}

fn compare_values(
    left: Value,
    right: Value,
    compare: OptionCompare,
    span: Span,
    predicate: impl FnOnce(std::cmp::Ordering) -> bool,
) -> Result<Value, Diagnostic> {
    let ordering = match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(&b),
        (Value::String(a), Value::String(b)) => {
            if compare == OptionCompare::Text {
                a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
            } else {
                a.cmp(&b)
            }
        }
        _ => {
            return Err(Diagnostic::new(
                "Comparison requires matching Integer or String operands",
                Some(span),
            ));
        }
    };

    Ok(Value::Boolean(predicate(ordering)))
}

pub(crate) fn values_equal(left: &Value, right: &Value, compare: OptionCompare) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => {
            if compare == OptionCompare::Text {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        }
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Empty, Value::Empty) => true,
        _ => false,
    }
}

fn values_identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Nothing, Value::Nothing) => true,
        (Value::Object(left), Value::Object(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

pub(crate) fn default_value(
    ty: &TypeName,
    types: &HashMap<String, RuntimeType>,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(value) = ty.builtin_default_value() {
        return Ok(value);
    }

    let TypeName::User(name) = ty else {
        unreachable!("builtin types are handled above");
    };
    if enums.contains_key(&key(name)) {
        return Ok(Value::Integer(0));
    }
    let type_def = types
        .get(&key(name))
        .ok_or_else(|| Diagnostic::new(format!("Type '{}' is not defined", name), Some(span)));
    let Ok(type_def) = type_def else {
        return Ok(Value::Nothing);
    };

    let mut fields = HashMap::new();
    for field in &type_def.fields {
        fields.insert(
            key(&field.name),
            default_value(&field.ty, types, enums, span)?,
        );
    }

    Ok(Value::Record {
        type_name: type_def.name.clone(),
        fields,
    })
}
pub(crate) fn coerce_assignment(
    ty: &TypeName,
    value: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    if matches!(value, Value::Missing) {
        return Err(Diagnostic::new(
            "Missing optional argument cannot be used as a value",
            Some(span),
        ));
    }
    if matches!(value, Value::Nothing) && matches!(ty, TypeName::User(_)) {
        return Ok(value);
    }
    if matches!(ty, TypeName::User(_)) && matches!(value, Value::Integer(_)) {
        return Ok(value);
    }
    if ty.same_type(&TypeName::Variant) || ty.same_type(&value.type_name()) {
        Ok(value)
    } else {
        Err(Diagnostic::new(
            format!(
                "Cannot assign {} value to {} variable",
                value.type_name().display_name(),
                ty.display_name()
            ),
            Some(span),
        )
        .with_primary_label(format!(
            "expected {}, found {}",
            ty.display_name(),
            value.type_name().display_name()
        ))
        .with_help("change the variable type or assign a value with the expected type"))
    }
}

pub(crate) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
