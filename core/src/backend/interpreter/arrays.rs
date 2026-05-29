use crate::runtime::{ArrayBound, Diagnostic, Span, Value, coerce_assignment};
use std::rc::Rc;

use super::values::{default_value, key};
use super::{Frame, Interpreter};

pub(crate) fn read_array_element(
    value: &Value,
    indices: &[i64],
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Value is not an array",
            Some(span),
        ));
    };
    ensure_allocated(array.allocated, span)?;
    let index = calculate_index(indices, &array.bounds, span)?;
    Ok(array.elements[index].clone())
}

pub(crate) fn write_array_element(
    value: &mut Value,
    indices: &[i64],
    new_value: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Value is not an array",
            Some(span),
        ));
    };
    let array = Rc::make_mut(array);
    ensure_allocated(array.allocated, span)?;
    let index = calculate_index(indices, &array.bounds, span)?;
    let old = array.elements[index].clone();
    array.elements[index] = coerce_assignment(&array.element_type, new_value, span)?;
    Ok(old)
}

pub(crate) fn array_element_mut<'a>(
    value: &'a mut Value,
    indices: &[i64],
    span: Span,
) -> Result<&'a mut Value, Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Value is not an array",
            Some(span),
        ));
    };
    let array = Rc::make_mut(array);
    ensure_allocated(array.allocated, span)?;
    let index = calculate_index(indices, &array.bounds, span)?;
    Ok(&mut array.elements[index])
}

pub(crate) fn lbound(value: &Value, dimension: usize, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "LBound requires an array",
            Some(span),
        ));
    };
    ensure_allocated(array.allocated, span)?;
    if dimension == 0 || dimension > array.bounds.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            format!("Array dimension {} is out of range", dimension),
            Some(span),
        ));
    }
    Ok(array.bounds[dimension - 1].lower)
}

pub(crate) fn ubound(value: &Value, dimension: usize, span: Span) -> Result<i64, Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "UBound requires an array",
            Some(span),
        ));
    };
    ensure_allocated(array.allocated, span)?;
    if dimension == 0 || dimension > array.bounds.len() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            format!("Array dimension {} is out of range", dimension),
            Some(span),
        ));
    }
    Ok(array.bounds[dimension - 1].upper)
}

pub(crate) fn redim_array(
    value: &mut Value,
    new_bounds: Vec<ArrayBound>,
    preserve: bool,
    interpreter: &Interpreter,
    span: Span,
) -> Result<(), Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "ReDim target must be a dynamic array",
            Some(span),
        )
        .with_primary_label("ReDim target is not a dynamic array"));
    };
    let array = Rc::make_mut(array);

    if preserve && array.allocated {
        if array.bounds.len() != new_bounds.len() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::ARRAY,
                "ReDim Preserve cannot change the number of dimensions",
                Some(span),
            ));
        }
        for (i, (old_bound, new_bound)) in array.bounds.iter().zip(&new_bounds).enumerate() {
            if i < array.bounds.len() - 1 {
                if old_bound != new_bound {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "ReDim Preserve can only change the last dimension",
                        Some(span),
                    ));
                }
            } else {
                // Last dimension
                if old_bound.lower != new_bound.lower {
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
    if preserve && array.allocated {
        // Copy existing elements that fit into the new bounds.
        // Since only the last dimension changes, and we use column-major,
        // the existing elements for smaller indices of the last dimension
        // are at the same physical indices in the flattened vector.
        let old_len = array.elements.len();
        new_elements.extend(array.elements.iter().take(new_len.min(old_len)).cloned());
    }

    while new_elements.len() < new_len {
        new_elements.push(default_value(&array.element_type, interpreter, span)?);
    }

    array.elements = new_elements;
    array.bounds = new_bounds;
    array.allocated = true;
    Ok(())
}

pub(crate) fn array_values(value: &Value, span: Span) -> Result<Vec<Value>, Diagnostic> {
    let Value::Array(array) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "For Each requires an array",
            Some(span),
        ));
    };
    ensure_allocated(array.allocated, span)?;
    Ok(array.elements.clone())
}

pub(crate) fn enumerable_values(
    interpreter: &mut Interpreter,
    value: Value,
    frame: &mut Frame,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    enumerable_values_with_depth(interpreter, value, frame, span, 0)
}

fn enumerable_values_with_depth(
    interpreter: &mut Interpreter,
    value: Value,
    frame: &mut Frame,
    span: Span,
    depth: usize,
) -> Result<Vec<Value>, Diagnostic> {
    if depth > 16 {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "Enumerable object chain is too deep",
            Some(span),
        ));
    }

    if matches!(value, Value::Array(_)) {
        return array_values(&value, span);
    }

    if let Value::ComObject(ref com_obj) = value {
        return crate::runtime::com::enumerable_com_values(com_obj, span);
    }

    if let Value::Collection(ref collection) = value {
        return Ok(collection
            .borrow()
            .items
            .iter()
            .map(|item| item.value.clone())
            .collect());
    }

    let Value::Object(object) = value.clone() else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ARRAY,
            "For Each requires an array, Variant array, or enumerable object",
            Some(span),
        ));
    };
    let class_name = object.borrow().class_name.clone();
    let class = interpreter
        .classes
        .get(&key(&class_name))
        .cloned()
        .ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Class '{}' is not defined", class_name),
                Some(span),
            )
        })?;

    if let Some(iterator) = class.iterator {
        let returned = interpreter.call_method_function_decl(value, iterator, frame, span)?;
        return enumerable_values_with_depth(interpreter, returned, frame, span, depth + 1)
            .map_err(|diagnostic| {
                if diagnostic.message.contains("For Each requires") {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        format!(
                            "Iterator for Class '{}' did not return an enumerable value",
                            class.name
                        ),
                        Some(span),
                    )
                } else {
                    diagnostic
                }
            });
    }

    if let Some(member) = class.enumerator_member {
        let returned = if class.functions.contains_key(&key(&member)) {
            interpreter.call_method_function(value, &member, &[], frame, span)?
        } else {
            interpreter.call_property_get(value, &member, &[], frame, span)?
        };
        return enumerable_values_with_depth(interpreter, returned, frame, span, depth + 1)
            .map_err(|diagnostic| {
                if diagnostic.message.contains("For Each requires") {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        format!(
                            "VB_UserMemId = -4 enumerator '{}' for Class '{}' did not return an enumerable value",
                            member, class.name
                        ),
                        Some(span),
                    )
                } else {
                    diagnostic
                }
            });
    }

    Err(Diagnostic::new(
        crate::runtime::DiagnosticCode::ARRAY,
        format!(
            "Class '{}' is not enumerable; define an Iterator or a VB_UserMemId = -4 _NewEnum member",
            class.name
        ),
        Some(span),
    ))
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
pub(crate) fn calculate_index(
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
                "Array index out of bounds",
                Some(span),
            )
            .with_primary_label("array index is outside the valid bounds")
            .with_note(format!(
                "index {} is outside valid range {} To {} for dimension {}",
                index,
                bound.lower,
                bound.upper,
                i + 1
            )));
        }

        flattened_index += (index - bound.lower) * multiplier;
        multiplier *= bound.upper - bound.lower + 1;
    }

    Ok(flattened_index as usize)
}
