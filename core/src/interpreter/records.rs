use crate::TypeDecl;
use crate::runtime::{Diagnostic, Span, TypeName, Value};

use super::values::{coerce_assignment, key};

pub(crate) fn read_field_member(
    value: &Value,
    field: &str,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Value::Object(object) = value {
        let object = object.borrow();
        return object.fields.get(&key(field)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            )
        });
    }
    if matches!(value, Value::Nothing) {
        return Err(Diagnostic::new("Object reference is Nothing", Some(span)));
    }
    let Value::Record { type_name, fields } = value else {
        return Err(Diagnostic::new(
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    };

    fields.get(&key(field)).cloned().ok_or_else(|| {
        Diagnostic::new(
            format!("Type '{}' has no field '{}'", type_name, field),
            Some(span),
        )
    })
}

pub(crate) fn write_member(
    value: &mut Value,
    field: &str,
    new_value: Value,
    span: Span,
) -> Result<(), Diagnostic> {
    if let Value::Object(object) = value {
        let mut object = object.borrow_mut();
        let Some(slot) = object.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            ));
        };
        let ty = slot.type_name();
        *slot = coerce_assignment(&ty, new_value, span)?;
        return Ok(());
    }
    if matches!(value, Value::Nothing) {
        return Err(Diagnostic::new("Object reference is Nothing", Some(span)));
    }
    let Value::Record { type_name, fields } = value else {
        return Err(Diagnostic::new(
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    };

    let Some(slot) = fields.get_mut(&key(field)) else {
        return Err(Diagnostic::new(
            format!("Type '{}' has no field '{}'", type_name, field),
            Some(span),
        ));
    };

    let ty = slot.type_name();
    *slot = coerce_assignment(&ty, new_value, span)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeType {
    pub(crate) name: String,
    pub(crate) fields: Vec<RuntimeField>,
}

impl From<&TypeDecl> for RuntimeType {
    fn from(value: &TypeDecl) -> Self {
        Self {
            name: value.name.clone(),
            fields: value
                .fields
                .iter()
                .map(|field| RuntimeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeField {
    pub(crate) name: String,
    pub(crate) ty: TypeName,
}
