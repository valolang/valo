use crate::runtime::numeric::{
    is_integer_type, is_really_float, value_to_f64, value_to_i64, value_to_u64,
};
use crate::runtime::{Diagnostic, Span, TypeName, Value};

pub fn coerce_assignment(ty: &TypeName, value: Value, span: Span) -> Result<Value, Diagnostic> {
    if matches!(value, Value::Missing) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "Optional argument was omitted here and cannot be forwarded into a required value",
            Some(span),
        ));
    }
    if matches!(value, Value::Nothing) && matches!(ty, TypeName::User(_)) {
        return Ok(value);
    }
    if matches!(ty, TypeName::User(name) if name.rsplit('.').next().is_some_and(|name| name.eq_ignore_ascii_case("Object")))
        && matches!(
            value,
            Value::Object(_) | Value::ComObject(_) | Value::Nothing
        )
    {
        return Ok(value);
    }
    if matches!(ty, TypeName::User(_)) && matches!(value, Value::Object(_) | Value::Nothing) {
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

pub fn type_mismatch_err(ty: &TypeName, value: &Value, span: Span) -> Diagnostic {
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

pub fn overflow_err(span: Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::RUNTIME,
        "Overflow",
        Some(span),
    )
}
