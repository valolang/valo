use std::collections::HashMap;
use std::rc::Rc;

use crate::BinaryOp;
use crate::runtime::{Diagnostic, Span, TypeName, Value};

use super::records::{RuntimeEnum, RuntimeType};

pub(crate) fn eval_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
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
        BinaryOp::Equal => Ok(Value::Boolean(values_equal(&left, &right))),
        BinaryOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right))),
        BinaryOp::Is => Ok(Value::Boolean(values_identical(&left, &right))),
        BinaryOp::Less => compare_values(left, right, span, |ord| ord.is_lt()),
        BinaryOp::Greater => compare_values(left, right, span, |ord| ord.is_gt()),
        BinaryOp::LessEqual => compare_values(left, right, span, |ord| ord.is_le()),
        BinaryOp::GreaterEqual => compare_values(left, right, span, |ord| ord.is_ge()),
    }
}

pub(crate) fn compare_case_values(
    left: Value,
    op: crate::CaseCompareOp,
    right: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    match op {
        crate::CaseCompareOp::Equal => Ok(Value::Boolean(values_equal(&left, &right))),
        crate::CaseCompareOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right))),
        crate::CaseCompareOp::Less => compare_values(left, right, span, |ord| ord.is_lt()),
        crate::CaseCompareOp::Greater => compare_values(left, right, span, |ord| ord.is_gt()),
        crate::CaseCompareOp::LessEqual => compare_values(left, right, span, |ord| ord.is_le()),
        crate::CaseCompareOp::GreaterEqual => compare_values(left, right, span, |ord| ord.is_ge()),
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
    span: Span,
    predicate: impl FnOnce(std::cmp::Ordering) -> bool,
) -> Result<Value, Diagnostic> {
    let ordering = match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(&b),
        (Value::String(a), Value::String(b)) => a.cmp(&b),
        _ => {
            return Err(Diagnostic::new(
                "Comparison requires matching Integer or String operands",
                Some(span),
            ));
        }
    };

    Ok(Value::Boolean(predicate(ordering)))
}

pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => a == b,
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
        ))
    }
}

pub(crate) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
