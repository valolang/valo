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
        BinaryOp::Add => math_binary(left, right, span, |a, b| a.wrapping_add(b), |a, b| a + b),
        BinaryOp::Subtract => {
            math_binary(left, right, span, |a, b| a.wrapping_sub(b), |a, b| a - b)
        }
        BinaryOp::Multiply => {
            math_binary(left, right, span, |a, b| a.wrapping_mul(b), |a, b| a * b)
        }
        BinaryOp::Divide => {
            let (a, b) = expect_numbers(left, right, span)?;
            if b == 0.0 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Division by zero",
                    Some(span),
                ));
            }
            Ok(Value::Double(a / b))
        }
        BinaryOp::IntegerDivide => {
            let (a, b) = expect_integers(left, right, span)?;
            if b == 0 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Division by zero",
                    Some(span),
                ));
            }
            Ok(Value::Int64(a / b))
        }
        BinaryOp::Modulo => {
            let (a, b) = expect_integers(left, right, span)?;
            if b == 0 {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Modulo by zero",
                    Some(span),
                ));
            }
            Ok(Value::Int64(a % b))
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
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Like requires String operands",
            Some(span),
        ));
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

fn math_binary(
    left: Value,
    right: Value,
    span: Span,
    int_op: impl FnOnce(i64, i64) -> i64,
    double_op: impl FnOnce(f64, f64) -> f64,
) -> Result<Value, Diagnostic> {
    if is_float_promotion(&left) || is_float_promotion(&right) {
        let (a, b) = expect_numbers(left, right, span)?;
        Ok(Value::Double(double_op(a, b)))
    } else {
        let (a, b) = expect_integers(left, right, span)?;
        Ok(Value::Int64(int_op(a, b)))
    }
}

fn is_float_promotion(v: &Value) -> bool {
    matches!(
        v,
        Value::Double(_)
            | Value::Single(_)
            | Value::Date(_)
            | Value::Currency(_)
            | Value::Decimal(_)
    )
}

fn expect_integers(left: Value, right: Value, span: Span) -> Result<(i64, i64), Diagnostic> {
    let a = value_to_i64(&left).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Operation requires integer operand, found {}",
                left.type_name().display_name()
            ),
            Some(span),
        )
    })?;
    let b = value_to_i64(&right).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Operation requires integer operand, found {}",
                right.type_name().display_name()
            ),
            Some(span),
        )
    })?;
    Ok((a, b))
}

fn expect_numbers(left: Value, right: Value, span: Span) -> Result<(f64, f64), Diagnostic> {
    let a = value_to_f64(&left).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Operation requires numeric operand, found {}",
                left.type_name().display_name()
            ),
            Some(span),
        )
    })?;
    let b = value_to_f64(&right).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Operation requires numeric operand, found {}",
                right.type_name().display_name()
            ),
            Some(span),
        )
    })?;
    Ok((a, b))
}

pub(crate) fn value_to_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Byte(n) => Some(*n as i64),
        Value::Int16(n) => Some(*n as i64),
        Value::Int32(n) => Some(*n as i64),
        Value::Int64(n) => Some(*n),
        Value::UInt32(n) => Some(*n as i64),
        Value::UInt64(n) => Some(*n as i64),
        Value::Single(n) => Some(*n as i64),
        Value::Double(n) => Some(*n as i64),
        Value::Currency(n) => Some(*n / 10000),
        Value::Decimal(n) => Some(*n as i64),
        Value::Boolean(b) => Some(if *b { -1 } else { 0 }),
        Value::Date(n) => Some(*n as i64),
        Value::Ptr(n) => Some(*n as i64),
        Value::FuncPtr(n) => Some(*n as i64),
        _ => None,
    }
}

pub(crate) fn value_to_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Byte(n) => Some(*n as u64),
        Value::Int16(n) if *n >= 0 => Some(*n as u64),
        Value::Int32(n) if *n >= 0 => Some(*n as u64),
        Value::Int64(n) if *n >= 0 => Some(*n as u64),
        Value::UInt32(n) => Some(*n as u64),
        Value::UInt64(n) => Some(*n),
        Value::Single(n) if *n >= 0.0 && *n <= u64::MAX as f32 => Some(*n as u64),
        Value::Double(n) if *n >= 0.0 && *n <= u64::MAX as f64 => Some(*n as u64),
        Value::Currency(n) if *n >= 0 => Some((*n as f64 / 10000.0) as u64),
        Value::Decimal(n) if *n >= 0 && *n <= u64::MAX as i128 => Some(*n as u64),
        Value::Boolean(b) => Some(if *b { 1 } else { 0 }),
        Value::Ptr(n) => Some(*n as u64),
        Value::FuncPtr(n) => Some(*n as u64),
        _ => None,
    }
}

pub(crate) fn value_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Byte(n) => Some(*n as f64),
        Value::Int16(n) => Some(*n as f64),
        Value::Int32(n) => Some(*n as f64),
        Value::Int64(n) => Some(*n as f64),
        Value::UInt32(n) => Some(*n as f64),
        Value::UInt64(n) => Some(*n as f64),
        Value::Single(n) => Some(*n as f64),
        Value::Double(n) => Some(*n),
        Value::Currency(n) => Some(*n as f64 / 10000.0),
        Value::Decimal(n) => Some(*n as f64),
        Value::Boolean(b) => Some(if *b { -1.0 } else { 0.0 }),
        Value::Date(n) => Some(*n),
        Value::Ptr(n) => Some(*n as f64),
        Value::FuncPtr(n) => Some(*n as f64),
        _ => None,
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
        (l, r) => {
            if let (Some(a), Some(b)) = (value_to_i64(&l), value_to_i64(&r)) {
                Ok(Value::Int64(int_op(a, b)))
            } else {
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Logical operators require Boolean or Integer operands",
                    Some(span),
                ))
            }
        }
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
        (Value::String(a), Value::String(b)) => {
            if compare == OptionCompare::Text {
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

pub(crate) fn values_equal(left: &Value, right: &Value, compare: OptionCompare) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => {
            if compare == OptionCompare::Text {
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
    if name.eq_ignore_ascii_case("Object") {
        return Ok(Value::Nothing);
    }
    if enums.contains_key(&key(name)) {
        return Ok(Value::Int64(0));
    }
    let type_def = types.get(&key(name)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", name),
            Some(span),
        )
    });
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
            crate::runtime::DiagnosticCode::GENERIC,
            "Missing optional argument cannot be used as a value",
            Some(span),
        ));
    }
    if matches!(value, Value::Nothing) && matches!(ty, TypeName::User(_)) {
        return Ok(value);
    }
    if matches!(ty, TypeName::User(name) if name.rsplit('.').next().is_some_and(|name| name.eq_ignore_ascii_case("Object")))
        && matches!(value, Value::Object(_) | Value::Nothing)
    {
        return Ok(value);
    }

    if matches!(ty, TypeName::User(_)) && is_integer_type(&value) {
        return Ok(value);
    }

    if ty.same_type(&TypeName::Variant) {
        return Ok(value);
    }

    if ty.same_type(&value.type_name()) {
        return Ok(value);
    }

    match ty {
        TypeName::Byte => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            if !(0..=255).contains(&v) {
                return Err(overflow_err(span));
            }
            Ok(Value::Byte(v as u8))
        }
        TypeName::Integer => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            if !(i16::MIN as i64..=i16::MAX as i64).contains(&v) {
                return Err(overflow_err(span));
            }
            Ok(Value::Int16(v as i16))
        }
        TypeName::Long => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            if !(i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                return Err(overflow_err(span));
            }
            Ok(Value::Int32(v as i32))
        }
        TypeName::Int64 => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::Int64(v))
        }
        TypeName::UInt32 => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            if !(0..=u32::MAX as i64).contains(&v) {
                return Err(overflow_err(span));
            }
            Ok(Value::UInt32(v as u32))
        }
        TypeName::UInt64 => {
            let v = value_to_u64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::UInt64(v))
        }
        TypeName::Single => {
            let v = value_to_f64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::Single(v as f32))
        }
        TypeName::Double => {
            let v = value_to_f64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::Double(v))
        }
        TypeName::Currency => {
            if is_really_float(&value) {
                let v = value_to_f64(&value).unwrap();
                Ok(Value::Currency((v * 10000.0) as i64))
            } else {
                let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
                Ok(Value::Currency(
                    v.checked_mul(10000).ok_or_else(|| overflow_err(span))?,
                ))
            }
        }
        TypeName::Decimal => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::Decimal(v as i128))
        }
        TypeName::Boolean => Ok(Value::Boolean(value.is_truthy())),
        TypeName::Date => {
            let v = value_to_f64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::Date(v))
        }
        TypeName::String => Ok(Value::String(value.to_output_string())),
        TypeName::Ptr => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::Ptr(v as usize))
        }
        TypeName::FuncPtr => {
            let v = value_to_i64(&value).ok_or_else(|| type_mismatch_err(ty, &value, span))?;
            Ok(Value::FuncPtr(v as usize))
        }
        _ => {
            if ty.same_type(&value.type_name()) {
                Ok(value)
            } else {
                Err(type_mismatch_err(ty, &value, span))
            }
        }
    }
}

fn is_really_float(v: &Value) -> bool {
    matches!(v, Value::Double(_) | Value::Single(_) | Value::Date(_))
}

fn is_integer_type(v: &Value) -> bool {
    matches!(
        v,
        Value::Byte(_)
            | Value::Int16(_)
            | Value::Int32(_)
            | Value::Int64(_)
            | Value::UInt32(_)
            | Value::UInt64(_)
            | Value::Ptr(_)
            | Value::FuncPtr(_)
    )
}

fn type_mismatch_err(ty: &TypeName, value: &Value, span: Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
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
    .with_help("change the variable type or assign a value with the expected type")
}

fn overflow_err(span: Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::RUNTIME,
        "Overflow",
        Some(span),
    )
}

pub(crate) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
