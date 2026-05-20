use crate::runtime::{Diagnostic, Value};
use crate::Expr;
use super::super::Frame;
use super::super::Interpreter;
use super::expect_arg_count;

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
        return Ok(Some(Value::Integer(vartype(&value))));
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

    Ok(None)
}

fn vartype(value: &Value) -> i64 {
    match value {
        Value::Empty => 0,
        Value::Null => 1,
        Value::Integer(_) => 2,
        Value::Double(_) => 5,
        Value::String(_) => 8,
        Value::Object(_) | Value::Nothing => 9,
        Value::Boolean(_) => 11,
        Value::Array { .. } => 8192,
        Value::Record { .. } | Value::Missing => 12,
    }
}

fn value_type_name(value: &Value) -> String {
    match value {
        Value::Empty => "Empty".to_string(),
        Value::Null => "Null".to_string(),
        Value::Integer(_) => "Integer".to_string(),
        Value::Double(_) => "Double".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Object(object) => object.borrow().class_name.clone(),
        Value::Nothing => "Nothing".to_string(),
        Value::Boolean(_) => "Boolean".to_string(),
        Value::Array { .. } => "Array".to_string(),
        Value::Record { type_name, .. } => type_name.clone(),
        Value::Missing => "Missing".to_string(),
    }
}
