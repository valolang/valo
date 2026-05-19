use crate::runtime::{Diagnostic, Span, Value};

use super::records::{RuntimeEnum, RuntimeType};
use super::values::{coerce_assignment, default_value};
use std::collections::HashMap;

pub(crate) fn read_array_element(
    value: &Value,
    index: i64,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array {
        elements,
        lower_bound,
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Value is not an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    let index = checked_index(index, *lower_bound, elements.len(), span)?;
    Ok(elements[index].clone())
}

pub(crate) fn write_array_element(
    value: &mut Value,
    index: i64,
    new_value: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array {
        element_type,
        elements,
        lower_bound,
        allocated,
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Value is not an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    let index = checked_index(index, *lower_bound, elements.len(), span)?;
    let old = elements[index].clone();
    elements[index] = coerce_assignment(element_type, new_value, span)?;
    Ok(old)
}

pub(crate) fn array_element_mut(
    value: &mut Value,
    index: i64,
    span: Span,
) -> Result<&mut Value, Diagnostic> {
    let Value::Array {
        elements,
        lower_bound,
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Value is not an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    let index = checked_index(index, *lower_bound, elements.len(), span)?;
    Ok(&mut elements[index])
}

pub(crate) fn lbound(value: &Value, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array {
        allocated,
        lower_bound,
        ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "LBound requires an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    Ok(*lower_bound)
}

pub(crate) fn ubound(value: &Value, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array {
        elements,
        lower_bound,
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "UBound requires an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    Ok(*lower_bound + elements.len() as i64 - 1)
}

pub(crate) fn redim_array(
    value: &mut Value,
    upper_bound: i64,
    lower_bound: i64,
    preserve: bool,
    types: &HashMap<String, RuntimeType>,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<(), Diagnostic> {
    if upper_bound < lower_bound {
        let message = if lower_bound == 0 {
            "ReDim upper bound must be non-negative"
        } else {
            "ReDim upper bound must be greater than or equal to the array lower bound"
        };
        return Err(
            Diagnostic::new(crate::runtime::DiagnosticCode::GENERIC, message, Some(span))
                .with_primary_label("invalid ReDim upper bound"),
        );
    }
    let Value::Array {
        element_type,
        elements,
        lower_bound: array_lower_bound,
        allocated,
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "ReDim target must be a dynamic array",
            Some(span),
        )
        .with_primary_label("ReDim target is not a dynamic array"));
    };
    let new_len = (upper_bound - lower_bound + 1) as usize;
    let mut new_elements = Vec::new();
    if preserve && *allocated {
        new_elements.extend(elements.iter().take(new_len).cloned());
    }
    while new_elements.len() < new_len {
        new_elements.push(default_value(element_type, types, enums, span)?);
    }
    *elements = new_elements;
    *array_lower_bound = lower_bound;
    *allocated = true;
    Ok(())
}

pub(crate) fn array_values(value: &Value, span: Span) -> Result<Vec<Value>, Diagnostic> {
    let Value::Array {
        elements,
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "For Each requires an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    Ok(elements.clone())
}

fn ensure_allocated(allocated: bool, span: Span) -> Result<(), Diagnostic> {
    if allocated {
        Ok(())
    } else {
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Dynamic array is unallocated",
            Some(span),
        ))
    }
}

fn checked_index(
    index: i64,
    lower_bound: i64,
    len: usize,
    span: Span,
) -> Result<usize, Diagnostic> {
    let offset = index - lower_bound;
    if index < lower_bound || offset as usize >= len {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            if lower_bound == 0 {
                format!("Array index {} is out of bounds for length {}", index, len)
            } else {
                format!(
                    "Array index {} is out of bounds for range {} to {}",
                    index,
                    lower_bound,
                    lower_bound + len as i64 - 1
                )
            },
            Some(span),
        )
        .with_primary_label("array index is outside the valid bounds"));
    }
    Ok(offset as usize)
}
