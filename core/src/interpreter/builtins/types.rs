use super::super::Frame;
use super::super::Interpreter;
use super::expect_arg_count;
use crate::Expr;
use crate::runtime::{Diagnostic, TypeName, Value};

pub(crate) fn eval_types(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Expr],
    frame: &mut Frame,
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("IsObject") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Boolean(matches!(
            value,
            Value::Object(_) | Value::Nothing
        ))));
    }
    if name.eq_ignore_ascii_case("IsArray") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Boolean(matches!(value, Value::Array { .. }))));
    }
    if name.eq_ignore_ascii_case("IsNull") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Boolean(matches!(value, Value::Null))));
    }
    if name.eq_ignore_ascii_case("IsEmpty") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Boolean(matches!(value, Value::Empty))));
    }
    if name.eq_ignore_ascii_case("IsError") {
        expect_arg_count(name, args, 1, span)?;
        return Ok(Some(Value::Boolean(false)));
    }
    if name.eq_ignore_ascii_case("VarType") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::Int64(vartype(&value))));
    }
    if name.eq_ignore_ascii_case("TypeName") {
        expect_arg_count(name, args, 1, span)?;
        let value = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(Value::String(value_type_name(&value))));
    }
    if name.eq_ignore_ascii_case("IIf") {
        expect_arg_count(name, args, 3, span)?;
        let condition = interpreter.eval_expr(&args[0], frame)?.is_truthy();
        let value = if condition { &args[1] } else { &args[2] };
        return Ok(Some(interpreter.eval_expr(value, frame)?));
    }
    if name.eq_ignore_ascii_case("CByte") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Byte,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CInt") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Integer,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CLng") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Long,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CLngLng") || name.eq_ignore_ascii_case("CInt64") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Int64,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CSng") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Single,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CDbl") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Double,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CDec") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Decimal,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CCur") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Currency,
            val,
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CDate") {
        expect_arg_count(name, args, 1, span)?;
        let val = interpreter.eval_expr(&args[0], frame)?;
        return Ok(Some(super::super::values::coerce_assignment(
            &TypeName::Date,
            val,
            span,
        )?));
    }

    Ok(None)
}

fn vartype(value: &Value) -> i64 {
    match value {
        Value::Empty => 0,
        Value::Null => 1,
        Value::Int16(_) => 2,
        Value::Int32(_) => 3,
        Value::Single(_) => 4,
        Value::Double(_) => 5,
        Value::Currency(_) => 6,
        Value::Date(_) => 7,
        Value::String(_) => 8,
        Value::Object(_) | Value::Nothing => 9,
        Value::Boolean(_) => 11,
        Value::Decimal(_) => 14,
        Value::Byte(_) => 17,
        Value::Int64(_) => 20,
        Value::Array { .. } => 8192,
        Value::Record { .. } | Value::Missing => 12,
        Value::Ptr(_) | Value::UInt32(_) | Value::UInt64(_) | Value::FuncPtr(_) => 12,
    }
}

fn value_type_name(value: &Value) -> String {
    match value {
        Value::Empty => "Empty".to_string(),
        Value::Null => "Null".to_string(),
        Value::Int16(_) => "Integer".to_string(),
        Value::Int32(_) => "Long".to_string(),
        Value::Int64(_) => "LongLong".to_string(),
        Value::Single(_) => "Single".to_string(),
        Value::Double(_) => "Double".to_string(),
        Value::Currency(_) => "Currency".to_string(),
        Value::Decimal(_) => "Decimal".to_string(),
        Value::Byte(_) => "Byte".to_string(),
        Value::Boolean(_) => "Boolean".to_string(),
        Value::Date(_) => "Date".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Object(object) => object.borrow().class_name.clone(),
        Value::Nothing => "Nothing".to_string(),
        Value::Array { .. } => "Array".to_string(),
        Value::Record { type_name, .. } => type_name.clone(),
        Value::Missing => "Missing".to_string(),
        Value::Ptr(_) => "Ptr".to_string(),
        Value::FuncPtr(_) => "FuncPtr".to_string(),
        Value::UInt32(_) => "UInt32".to_string(),
        Value::UInt64(_) => "UInt64".to_string(),
    }
}
