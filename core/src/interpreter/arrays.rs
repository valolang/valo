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
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    ensure_allocated(*allocated, span)?;
    let index = checked_index(index, elements.len(), span)?;
    Ok(elements[index].clone())
}

pub(crate) fn write_array_element(
    value: &mut Value,
    index: i64,
    new_value: Value,
    span: Span,
) -> Result<(), Diagnostic> {
    let Value::Array {
        element_type,
        elements,
        allocated,
    } = value
    else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    ensure_allocated(*allocated, span)?;
    let index = checked_index(index, elements.len(), span)?;
    elements[index] = coerce_assignment(element_type, new_value, span)?;
    Ok(())
}

pub(crate) fn array_element_mut(
    value: &mut Value,
    index: i64,
    span: Span,
) -> Result<&mut Value, Diagnostic> {
    let Value::Array {
        elements,
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    ensure_allocated(*allocated, span)?;
    let index = checked_index(index, elements.len(), span)?;
    Ok(&mut elements[index])
}

pub(crate) fn lbound(value: &Value, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array { allocated, .. } = value else {
        return Err(Diagnostic::new("LBound requires an array", Some(span)));
    };
    ensure_allocated(*allocated, span)?;
    Ok(0)
}

pub(crate) fn ubound(value: &Value, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array {
        elements,
        allocated,
        ..
    } = value
    else {
        return Err(Diagnostic::new("UBound requires an array", Some(span)));
    };
    ensure_allocated(*allocated, span)?;
    Ok(elements.len() as i64 - 1)
}

pub(crate) fn redim_array(
    value: &mut Value,
    upper_bound: i64,
    preserve: bool,
    types: &HashMap<String, RuntimeType>,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<(), Diagnostic> {
    if upper_bound < 0 {
        return Err(Diagnostic::new(
            "ReDim upper bound must be non-negative",
            Some(span),
        ));
    }
    let Value::Array {
        element_type,
        elements,
        allocated,
    } = value
    else {
        return Err(Diagnostic::new(
            "ReDim target must be a dynamic array",
            Some(span),
        ));
    };
    let new_len = upper_bound as usize + 1;
    let mut new_elements = Vec::new();
    if preserve && *allocated {
        new_elements.extend(elements.iter().take(new_len).cloned());
    }
    while new_elements.len() < new_len {
        new_elements.push(default_value(element_type, types, enums, span)?);
    }
    *elements = new_elements;
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
        return Err(Diagnostic::new("For Each requires an array", Some(span)));
    };
    ensure_allocated(*allocated, span)?;
    Ok(elements.clone())
}

fn ensure_allocated(allocated: bool, span: Span) -> Result<(), Diagnostic> {
    if allocated {
        Ok(())
    } else {
        Err(Diagnostic::new("Dynamic array is unallocated", Some(span)))
    }
}

fn checked_index(index: i64, len: usize, span: Span) -> Result<usize, Diagnostic> {
    if index < 0 || index as usize >= len {
        return Err(Diagnostic::new(
            format!("Array index {} is out of bounds for length {}", index, len),
            Some(span),
        ));
    }
    Ok(index as usize)
}
