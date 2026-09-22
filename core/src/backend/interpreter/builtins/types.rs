//! Builtins that inspect or convert a value's type.
//!
//! Each is a named function reached through [`HANDLERS`], rather than a branch
//! in a chain of name comparisons. The table *is* the dispatch, so the set of
//! builtins this module implements can be read off, and checked against the
//! registry by a test, instead of being inferred by reading the code.
//!
//! Argument counts are checked against the registry before dispatch, so these
//! functions index the arguments their registry entry guarantees.

use super::{ValueFn, find_handler};
use crate::backend::interpreter::Interpreter;
use crate::runtime::{Diagnostic, Span, TypeName, Value, coerce_assignment, well_known};
use std::rc::Rc;

/// The builtins this module implements.
pub(super) const HANDLERS: &[(&str, ValueFn)] = &[
    ("IsObject", is_object),
    ("IsArray", is_array),
    ("IsNull", is_null),
    ("IsEmpty", is_empty),
    ("IsError", is_error),
    ("IsNumeric", is_numeric),
    ("IsDate", is_date),
    ("VarType", var_type),
    ("TypeName", type_name),
    ("CVar", c_var),
    ("CVErr", c_verr),
    ("CByte", c_byte),
    ("CInt", c_int),
    ("CLng", c_lng),
    ("CLngLng", c_lng_lng),
    ("CLngPtr", c_lng_lng),
    ("CInt64", c_lng_lng),
    ("CSng", c_sng),
    ("CDbl", c_dbl),
    ("CDec", c_dec),
    ("CCur", c_cur),
    ("CDate", c_date),
    ("CBool", c_bool),
];

pub(crate) fn eval_types(
    interpreter: &mut Interpreter,
    name: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match find_handler(HANDLERS, name) {
        Some(handler) => handler(interpreter, name, args, span).map(Some),
        None => Ok(None),
    }
}

fn is_object(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Boolean(matches!(
        args[0],
        Value::Object(_) | Value::Nothing
    )))
}

fn is_array(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Boolean(matches!(args[0], Value::Array(_))))
}

fn is_null(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Boolean(matches!(args[0], Value::Null)))
}

fn is_empty(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Boolean(matches!(args[0], Value::Empty)))
}

fn is_error(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Boolean(matches!(args[0], Value::Error(_))))
}

fn is_numeric(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    let numeric = match &args[0] {
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
        | Value::Boolean(_)
        | Value::Date(_)
        | Value::Empty => true,
        // A string counts as numeric when it would convert cleanly.
        Value::String(text) => text.trim().parse::<f64>().is_ok(),
        _ => false,
    };
    Ok(Value::Boolean(numeric))
}

fn is_date(_: &mut Interpreter, _: &str, args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let is_date = match &args[0] {
        Value::Date(_) => true,
        Value::String(text) => {
            super::parse_date_value(text, span).is_ok()
                || super::parse_time_value(text, span).is_ok()
        }
        _ => false,
    };
    Ok(Value::Boolean(is_date))
}

fn var_type(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::Int64(vartype(&args[0])))
}

fn type_name(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(Value::String(Rc::new(match_value_type_name(&args[0]))))
}

fn c_var(_: &mut Interpreter, _: &str, args: &[Value], _: Span) -> Result<Value, Diagnostic> {
    Ok(args[0].clone())
}

fn c_verr(_: &mut Interpreter, _: &str, args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let code = crate::runtime::numeric::value_to_i64(&args[0]).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "CVErr requires an integer error code",
            Some(span),
        )
    })?;
    Ok(Value::Error(code as i32))
}

/// Defines a conversion builtin that coerces its argument to a fixed type.
///
/// These differ only in their target type, so declaring them this way keeps each
/// one a single readable line and leaves no room for a body to drift from the
/// name it is registered under.
macro_rules! conversions {
    ($($fn_name:ident => $target:ident,)*) => {
        $(
            fn $fn_name(
                _: &mut Interpreter,
                _: &str,
                args: &[Value],
                span: Span,
            ) -> Result<Value, Diagnostic> {
                coerce_assignment(&TypeName::$target, args[0].clone(), span)
            }
        )*
    };
}

conversions! {
    c_byte => Byte,
    c_int => Int32,
    c_lng => Int64,
    c_lng_lng => Int64,
    c_sng => Single,
    c_dbl => Double,
    c_dec => Decimal,
    c_cur => Currency,
    c_date => Date,
    c_bool => Boolean,
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
        Value::Object(_) | Value::Nothing | Value::Collection(_) => 9,
        Value::Boolean(_) => 11,
        Value::Decimal(_) => 14,
        Value::Byte(_) => 17,
        Value::Int64(_) => 20,
        Value::Array(_) => 8192,
        Value::Error(_) => 10,
        Value::Record(_) | Value::Missing | Value::BoxedRecord(_, _) => 12,
        Value::UInt32(_) | Value::UInt64(_) => 12,
        Value::Ptr(_) | Value::FuncPtr(_) => {
            if cfg!(target_pointer_width = "64") {
                20
            } else {
                3
            }
        }
        Value::Nullable(value) => vartype(value),
        Value::Lambda(_) => 9,
    }
}

fn match_value_type_name(value: &Value) -> String {
    match value {
        Value::Empty => "Empty".to_string(),
        Value::Null => "Null".to_string(),
        Value::Int16(_) => "Short".to_string(),
        Value::Int32(_) => "Integer".to_string(),
        Value::Int64(_) => "Long".to_string(),
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
        Value::Collection(_) => well_known::COLLECTION.to_string(),
        Value::Nothing => "Nothing".to_string(),
        Value::Array(_) => "Array".to_string(),
        Value::Record(record) => record.type_name.clone(),
        Value::BoxedRecord(record, _) => record.type_name.clone(),
        Value::Missing => "Missing".to_string(),
        Value::Nullable(value) => match_value_type_name(value),
        Value::Lambda(_) => well_known::FUNC.to_string(),
        Value::UInt32(_) => "UInt32".to_string(),
        Value::UInt64(_) => "UInt64".to_string(),
        Value::Ptr(_) => "Ptr".to_string(),
        Value::FuncPtr(_) => "FuncPtr".to_string(),
    }
}
