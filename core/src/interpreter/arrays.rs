use crate::runtime::{Diagnostic, Span, Value};

use super::values::coerce_assignment;

pub(crate) fn read_array_element(
    value: &Value,
    index: i64,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array { elements, .. } = value else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
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
    } = value
    else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    let index = checked_index(index, elements.len(), span)?;
    elements[index] = coerce_assignment(element_type, new_value, span)?;
    Ok(())
}

pub(crate) fn array_element_mut(
    value: &mut Value,
    index: i64,
    span: Span,
) -> Result<&mut Value, Diagnostic> {
    let Value::Array { elements, .. } = value else {
        return Err(Diagnostic::new("Value is not an array", Some(span)));
    };
    let index = checked_index(index, elements.len(), span)?;
    Ok(&mut elements[index])
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
