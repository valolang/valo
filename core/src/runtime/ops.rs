use crate::runtime::compare::{
    RuntimeOptionCompare, compare_values, like_values, values_equal, values_identical,
};
use crate::runtime::numeric::{expect_integers, expect_numbers, logical_or_bitwise, math_binary};
use crate::runtime::{Diagnostic, Span, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBinaryOp {
    Add,
    Subtract,
    Multiply,
    Exponent,
    Divide,
    IntegerDivide,
    Modulo,
    Concat,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Is,
    Like,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

pub fn eval_binary(
    left: Value,
    op: RuntimeBinaryOp,
    right: Value,
    compare: RuntimeOptionCompare,
    span: Span,
) -> Result<Value, Diagnostic> {
    match op {
        RuntimeBinaryOp::Add => {
            math_binary(left, right, span, |a, b| a.wrapping_add(b), |a, b| a + b)
        }
        RuntimeBinaryOp::Subtract => {
            math_binary(left, right, span, |a, b| a.wrapping_sub(b), |a, b| a - b)
        }
        RuntimeBinaryOp::Multiply => {
            math_binary(left, right, span, |a, b| a.wrapping_mul(b), |a, b| a * b)
        }
        RuntimeBinaryOp::Exponent => {
            let (a, b) = expect_numbers(left, right, span)?;
            Ok(Value::Double(a.powf(b)))
        }
        RuntimeBinaryOp::Divide => {
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
        RuntimeBinaryOp::IntegerDivide => {
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
        RuntimeBinaryOp::Modulo => {
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
        RuntimeBinaryOp::Concat => Ok(Value::String(format!(
            "{}{}",
            left.to_output_string(),
            right.to_output_string()
        ))),
        RuntimeBinaryOp::LogicalAnd => {
            logical_or_bitwise(left, right, span, |a, b| a && b, |a, b| a & b)
        }
        RuntimeBinaryOp::LogicalOr => {
            logical_or_bitwise(left, right, span, |a, b| a || b, |a, b| a | b)
        }
        RuntimeBinaryOp::Equal => Ok(Value::Boolean(values_equal(&left, &right, compare))),
        RuntimeBinaryOp::NotEqual => Ok(Value::Boolean(!values_equal(&left, &right, compare))),
        RuntimeBinaryOp::Is => Ok(Value::Boolean(values_identical(&left, &right))),
        RuntimeBinaryOp::Like => like_values(left, right, compare, span),
        RuntimeBinaryOp::Less => compare_values(left, right, compare, span, |ord| ord.is_lt()),
        RuntimeBinaryOp::Greater => compare_values(left, right, compare, span, |ord| ord.is_gt()),
        RuntimeBinaryOp::LessEqual => compare_values(left, right, compare, span, |ord| ord.is_le()),
        RuntimeBinaryOp::GreaterEqual => {
            compare_values(left, right, compare, span, |ord| ord.is_ge())
        }
    }
}
