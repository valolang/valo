use crate::runtime::{Diagnostic, TypeName, Value, coerce_assignment};

pub(crate) fn eval_types(
    name: &str,
    args: &[Value],
    span: crate::runtime::Span,
) -> Result<Option<Value>, Diagnostic> {
    if name.eq_ignore_ascii_case("IsObject") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::Boolean(matches!(
            args[0],
            Value::Object(_) | Value::Nothing
        ))));
    }
    if name.eq_ignore_ascii_case("IsArray") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::Boolean(matches!(args[0], Value::Array(_)))));
    }
    if name.eq_ignore_ascii_case("IsNull") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::Boolean(matches!(args[0], Value::Null))));
    }
    if name.eq_ignore_ascii_case("IsEmpty") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::Boolean(matches!(args[0], Value::Empty))));
    }
    if name.eq_ignore_ascii_case("IsError") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::Boolean(matches!(args[0], Value::Error(_)))));
    }
    if name.eq_ignore_ascii_case("VarType") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::Int64(vartype(&args[0]))));
    }
    if name.eq_ignore_ascii_case("TypeName") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(Value::String(value_type_name(&args[0]))));
    }
    if name.eq_ignore_ascii_case("CByte") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Byte,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CInt") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Integer,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CLng") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Long,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CLngLng") || name.eq_ignore_ascii_case("CInt64") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Int64,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CSng") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Single,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CDbl") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Double,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CDec") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Decimal,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CCur") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Currency,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CDate") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Date,
            args[0].clone(),
            span,
        )?));
    }
    if name.eq_ignore_ascii_case("CBool") {
        expect_value_count(name, args, 1, span)?;
        return Ok(Some(coerce_assignment(
            &TypeName::Boolean,
            args[0].clone(),
            span,
        )?));
    }

    Ok(None)
}

fn expect_value_count(
    name: &str,
    args: &[Value],
    expected: usize,
    span: crate::runtime::Span,
) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            format!("{name} expects exactly {expected} argument(s)"),
            Some(span),
        ))
    }
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
        Value::Array(_) => 8192,
        Value::Error(_) => 10,
        Value::Record(_) | Value::Missing => 12,
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
        Value::Error(_) => "Error".to_string(),
        Value::Object(object) => object.borrow().class_name.clone(),
        Value::Nothing => "Nothing".to_string(),
        Value::Array(_) => "Array".to_string(),
        Value::Record(record) => record.type_name.clone(),
        Value::Missing => "Missing".to_string(),
        Value::Ptr(_) => "Ptr".to_string(),
        Value::FuncPtr(_) => "FuncPtr".to_string(),
        Value::UInt32(_) => "UInt32".to_string(),
        Value::UInt64(_) => "UInt64".to_string(),
    }
}
