use crate::runtime::{ArrayBound, Diagnostic, Span, Value};

use super::records::{RuntimeEnum, RuntimeType};
use super::values::{coerce_assignment, default_value};
use std::collections::HashMap;

pub(crate) fn read_array_element(
    value: &Value,
    indices: &[i64],
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array {
        elements,
        bounds,
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
    let index = calculate_index(indices, bounds, span)?;
    Ok(elements[index].clone())
}

pub(crate) fn write_array_element(
    value: &mut Value,
    indices: &[i64],
    new_value: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array {
        element_type,
        elements,
        bounds,
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
    let index = calculate_index(indices, bounds, span)?;
    let old = elements[index].clone();
    elements[index] = coerce_assignment(element_type, new_value, span)?;
    Ok(old)
}

pub(crate) fn array_element_mut<'a>(
    value: &'a mut Value,
    indices: &[i64],
    span: Span,
) -> Result<&'a mut Value, Diagnostic> {
    let Value::Array {
        elements,
        bounds,
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
    let index = calculate_index(indices, bounds, span)?;
    Ok(&mut elements[index])
}

pub(crate) fn lbound(value: &Value, dimension: usize, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array {
        allocated, bounds, ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "LBound requires an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    if dimension == 0 || dimension > bounds.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            format!("Array dimension {} is out of range", dimension),
            Some(span),
        ));
    }
    Ok(bounds[dimension - 1].lower)
}

pub(crate) fn ubound(value: &Value, dimension: usize, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array {
        allocated, bounds, ..
    } = value
    else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "UBound requires an array",
            Some(span),
        ));
    };
    ensure_allocated(*allocated, span)?;
    if dimension == 0 || dimension > bounds.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            format!("Array dimension {} is out of range", dimension),
            Some(span),
        ));
    }
    Ok(bounds[dimension - 1].upper)
}

pub(crate) fn redim_array(
    value: &mut Value,
    new_bounds: Vec<ArrayBound>,
    preserve: bool,
    types: &HashMap<String, RuntimeType>,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<(), Diagnostic> {
    let Value::Array {
        element_type,
        elements,
        bounds: old_bounds,
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

    if preserve && *allocated {
        if old_bounds.len() != new_bounds.len() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "ReDim Preserve cannot change the number of dimensions",
                Some(span),
            ));
        }
        for i in 0..old_bounds.len() {
            if i < old_bounds.len() - 1 {
                if old_bounds[i] != new_bounds[i] {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "ReDim Preserve can only change the last dimension",
                        Some(span),
                    ));
                }
            } else {
                // Last dimension
                if old_bounds[i].lower != new_bounds[i].lower {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "ReDim Preserve cannot change the lower bound of the last dimension",
                        Some(span),
                    ));
                }
            }
        }
    }

    let mut new_len: usize = 1;
    for bound in &new_bounds {
        new_len *= (bound.upper - bound.lower + 1) as usize;
    }

    let mut new_elements = Vec::new();
    if preserve && *allocated {
        // Copy existing elements that fit into the new bounds.
        // Since only the last dimension changes, and we use column-major,
        // the existing elements for smaller indices of the last dimension
        // are at the same physical indices in the flattened vector.
        let old_len = elements.len();
        new_elements.extend(elements.iter().take(new_len.min(old_len)).cloned());
    }

    while new_elements.len() < new_len {
        new_elements.push(default_value(element_type, types, enums, span)?);
    }

    *elements = new_elements;
    *old_bounds = new_bounds;
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

/// Calculate the flattened index for a multidimensional array (column-major).
/// column-major: index = d1 + D1 * (d2 + D2 * (d3 + ...))
/// where Di is the size of dimension i.
fn calculate_index(
    indices: &[i64],
    bounds: &[ArrayBound],
    span: Span,
) -> Result<usize, Diagnostic> {
    if indices.len() != bounds.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            format!(
                "Wrong number of dimensions: expected {}, got {}",
                bounds.len(),
                indices.len()
            ),
            Some(span),
        ));
    }

    let mut flattened_index: i64 = 0;
    let mut multiplier: i64 = 1;

    for (i, &index) in indices.iter().enumerate() {
        let bound = &bounds[i];
        if index < bound.lower || index > bound.upper {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                format!(
                    "Array index {} is out of bounds for dimension {} ({} to {})",
                    index,
                    i + 1,
                    bound.lower,
                    bound.upper
                ),
                Some(span),
            )
            .with_primary_label("array index is outside the valid bounds"));
        }

        flattened_index += (index - bound.lower) * multiplier;
        multiplier *= bound.upper - bound.lower + 1;
    }

    Ok(flattened_index as usize)
}
