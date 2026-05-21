use crate::runtime::{Diagnostic, Span, Value};

pub fn unary_positive(value: Value, span: Span) -> Result<Value, Diagnostic> {
    if is_numeric_value(&value) {
        Ok(value)
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Unary '+' requires a numeric expression, found {}",
                value.type_name().display_name()
            ),
            Some(span),
        ))
    }
}

pub fn unary_negate(value: Value, span: Span) -> Result<Value, Diagnostic> {
    match value {
        Value::Byte(value) => Ok(Value::Int16(-(value as i16))),
        Value::Int16(value) => Ok(Value::Int16(value.wrapping_neg())),
        Value::Int32(value) => Ok(Value::Int32(value.wrapping_neg())),
        Value::Int64(value) => Ok(Value::Int64(value.wrapping_neg())),
        Value::UInt32(value) => Ok(Value::Int64(-(value as i64))),
        Value::UInt64(value) if value <= i64::MAX as u64 => Ok(Value::Int64(-(value as i64))),
        Value::UInt64(value) => Ok(Value::Int64((value as i64).wrapping_neg())),
        Value::Single(value) => Ok(Value::Single(-value)),
        Value::Double(value) => Ok(Value::Double(-value)),
        Value::Currency(value) => Ok(Value::Currency(value.wrapping_neg())),
        Value::Decimal(value) => Ok(Value::Decimal(value.wrapping_neg())),
        Value::Date(value) => Ok(Value::Date(-value)),
        value => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            format!(
                "Unary '-' requires a numeric expression, found {}",
                value.type_name().display_name()
            ),
            Some(span),
        )),
    }
}

pub fn is_numeric_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Byte(_)
            | Value::Int16(_)
            | Value::Int32(_)
            | Value::Int64(_)
            | Value::UInt32(_)
            | Value::UInt64(_)
            | Value::Single(_)
            | Value::Double(_)
            | Value::Currency(_)
            | Value::Decimal(_)
            | Value::Date(_)
    )
}

pub fn math_binary(
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

pub fn is_float_promotion(v: &Value) -> bool {
    matches!(
        v,
        Value::Double(_)
            | Value::Single(_)
            | Value::Date(_)
            | Value::Currency(_)
            | Value::Decimal(_)
    )
}

pub fn expect_integers(left: Value, right: Value, span: Span) -> Result<(i64, i64), Diagnostic> {
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

pub fn expect_numbers(left: Value, right: Value, span: Span) -> Result<(f64, f64), Diagnostic> {
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

pub fn value_to_i64(v: &Value) -> Option<i64> {
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

pub fn value_to_u64(v: &Value) -> Option<u64> {
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

pub fn value_to_f64(v: &Value) -> Option<f64> {
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

pub fn logical_or_bitwise(
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

pub fn is_really_float(v: &Value) -> bool {
    matches!(v, Value::Double(_) | Value::Single(_) | Value::Date(_))
}

pub fn is_integer_type(v: &Value) -> bool {
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
