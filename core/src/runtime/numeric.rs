use crate::runtime::{Diagnostic, Span, Value};

fn parse_numeric_string_i64(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (sign, body) = if let Some(rest) = trimmed.strip_prefix('-') {
        (-1_i64, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (1_i64, rest)
    } else {
        (1_i64, trimmed)
    };

    if let Some(hex) = body.strip_prefix("&H").or_else(|| body.strip_prefix("&h")) {
        let value = i64::from_str_radix(hex, 16).ok()?;
        return value.checked_mul(sign);
    }
    if let Some(octal) = body.strip_prefix("&O").or_else(|| body.strip_prefix("&o")) {
        let value = i64::from_str_radix(octal, 8).ok()?;
        return value.checked_mul(sign);
    }

    trimmed.parse::<f64>().ok().map(|value| value as i64)
}

fn parse_numeric_string_f64(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(value) = parse_numeric_string_i64(trimmed)
        && (trimmed.contains("&H")
            || trimmed.contains("&h")
            || trimmed.contains("&O")
            || trimmed.contains("&o"))
    {
        return Some(value as f64);
    }

    trimmed.parse::<f64>().ok()
}

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

/// Converts a value to an integer the way Basic does: by rounding.
///
/// `CInt(3.9)` is 4, not 3. Ties go to the even neighbour, so `CInt(2.5)` is 2
/// and `CInt(3.5)` is 4. VB.NET and VBA agree on this, so there is nothing to
/// trade off between them. Use this wherever a conversion is what the program
/// asked for; [`value_to_i64`] truncates, which is right for reading an index
/// or a bit pattern out of a value that already holds one.
pub fn value_to_rounded_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Single(n) => Some(f64::from(*n).round_ties_even() as i64),
        Value::Double(n) | Value::Date(n) => Some(n.round_ties_even() as i64),
        // Currency is a scaled integer, so rounding it through `f64` would lose
        // the low digits of a large amount. Divide and round the remainder.
        Value::Currency(n) => Some(divide_ties_even(*n, 10_000)),
        Value::String(text) => parse_numeric_string_f64(text)
            .map(|value| value.round_ties_even() as i64)
            .or_else(|| parse_numeric_string_i64(text)),
        Value::Nullable(inner) => value_to_rounded_i64(inner),
        other => value_to_i64(other),
    }
}

/// Divides, rounding a tie to the even quotient.
fn divide_ties_even(numerator: i64, divisor: i64) -> i64 {
    let quotient = numerator / divisor;
    let remainder = (numerator % divisor).abs();
    let step = if numerator < 0 { -1 } else { 1 };

    match (remainder * 2).cmp(&divisor) {
        std::cmp::Ordering::Greater => quotient + step,
        std::cmp::Ordering::Equal if quotient % 2 != 0 => quotient + step,
        _ => quotient,
    }
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
        Value::String(text) => parse_numeric_string_i64(text),
        Value::Nothing | Value::Null | Value::Empty => Some(0),
        Value::Nullable(inner) => value_to_i64(inner),
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
        Value::Nothing | Value::Null | Value::Empty => Some(0),
        Value::Nullable(inner) => value_to_u64(inner),
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
        Value::String(text) => parse_numeric_string_f64(text),
        Value::Nothing | Value::Null | Value::Empty => Some(0.0),
        Value::Nullable(inner) => value_to_f64(inner),
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
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
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

/// Execute a frontend-selected primitive operation without value-dependent promotion.
pub(crate) fn typed_arithmetic(
    left: Value,
    right: Value,
    op: crate::frontend::semantics::arithmetic::ArithmeticOp,
    signature: &crate::frontend::semantics::arithmetic::ArithmeticSignature,
    span: Span,
) -> Result<Value, Diagnostic> {
    use crate::TypeName;
    use crate::frontend::semantics::arithmetic::ArithmeticOp as Op;
    let zero = || {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::ARITHMETIC,
            if op == Op::Modulo {
                "Modulo by zero"
            } else {
                "Division by zero"
            },
            Some(span),
        )
    };
    macro_rules! integer {
        ($a:expr, $b:expr, $variant:ident) => {{
            let (a, b) = ($a, $b);
            if b == 0 && matches!(op, Op::IntegerDivide | Op::Modulo) {
                return Err(zero());
            }
            let result = match op {
                Op::Add => a.wrapping_add(b),
                Op::Subtract => a.wrapping_sub(b),
                Op::Multiply => a.wrapping_mul(b),
                Op::IntegerDivide => a.wrapping_div(b),
                Op::Modulo => a.wrapping_rem(b),
                _ => unreachable!("floating result selected"),
            };
            Ok(Value::$variant(result))
        }};
    }
    macro_rules! floating {
        ($a:expr, $b:expr, $variant:ident) => {{
            let (a, b) = ($a, $b);
            if b == 0.0 && matches!(op, Op::Divide | Op::Modulo) {
                return Err(zero());
            }
            Ok(Value::$variant(match op {
                Op::Add => a + b,
                Op::Subtract => a - b,
                Op::Multiply => a * b,
                Op::Divide => a / b,
                Op::Modulo => a % b,
                Op::Power => a.powf(b),
                Op::IntegerDivide => unreachable!("integer result selected"),
            }))
        }};
    }
    match signature.result_type {
        TypeName::Int32 => {
            let (a, b) = expect_integers(left, right, span)?;
            integer!(a as i32, b as i32, Int32)
        }
        TypeName::Int64 => {
            let (a, b) = expect_integers(left, right, span)?;
            integer!(a, b, Int64)
        }
        TypeName::UInt32 => {
            let a = value_to_u64(&left).expect("selected numeric type") as u32;
            let b = value_to_u64(&right).expect("selected numeric type") as u32;
            integer!(a, b, UInt32)
        }
        TypeName::UInt64 => {
            let a = value_to_u64(&left).expect("selected numeric type");
            let b = value_to_u64(&right).expect("selected numeric type");
            integer!(a, b, UInt64)
        }
        TypeName::Single => {
            let (a, b) = expect_numbers(left, right, span)?;
            floating!(a as f32, b as f32, Single)
        }
        TypeName::Double => {
            let (a, b) = expect_numbers(left, right, span)?;
            floating!(a, b, Double)
        }
        _ => unreachable!("arithmetic signature only selects primitive numeric types"),
    }
}
