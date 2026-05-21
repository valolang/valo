use crate::runtime::{Diagnostic, Span, TypeName, Value, coerce_assignment};
use crate::{ClassMember, TypeDecl, TypeKind};
use std::rc::Rc;

use super::properties::{RuntimeProperty, RuntimePropertyAccessor};
use super::values::key;

pub(crate) fn read_field_member(
    value: &Value,
    field: &str,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Value::Object(object) = value {
        let object = object.borrow();
        return object.fields.get(&key(field)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            )
        });
    }
    if matches!(value, Value::Nothing) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            "Object reference is Nothing",
            Some(span),
        )
        .with_primary_label("attempted to access a member on Nothing")
        .with_help("assign an object before accessing its members"));
    }
    let Value::Record(record) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member access requires a user-defined Type value",
            Some(span),
        ));
    };

    record.fields.get(&key(field)).cloned().ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Type '{}' has no field '{}'", record.type_name, field),
            Some(span),
        )
    })
}

pub(crate) fn write_member(
    value: &mut Value,
    field: &str,
    new_value: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Value::Object(object) = value {
        let mut object = object.borrow_mut();
        let Some(slot) = object.fields.get_mut(&key(field)) else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no field '{}'", object.class_name, field),
                Some(span),
            ));
        };
        let ty = slot.type_name();
        let old = slot.clone();
        *slot = coerce_assignment(&ty, new_value, span)?;
        return Ok(old);
    }
    if matches!(value, Value::Nothing) {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            "Object reference is Nothing",
            Some(span),
        )
        .with_primary_label("attempted to assign a member on Nothing")
        .with_help("assign an object before assigning its members"));
    }
    let Value::Record(record) = value else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Member assignment requires a user-defined Type value",
            Some(span),
        ));
    };
    let record = Rc::make_mut(record);

    let Some(slot) = record.fields.get_mut(&key(field)) else {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!("Type '{}' has no field '{}'", record.type_name, field),
            Some(span),
        ));
    };

    let ty = slot.type_name();
    let old = slot.clone();
    *slot = coerce_assignment(&ty, new_value, span)?;
    Ok(old)
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeType {
    pub(crate) name: String,
    pub(crate) is_structure: bool,
    pub(crate) fields: Vec<RuntimeField>,
    pub(crate) subs: std::collections::HashMap<String, crate::Procedure>,
    pub(crate) functions: std::collections::HashMap<String, crate::Function>,
    pub(crate) properties: std::collections::HashMap<String, RuntimeProperty>,
    pub(crate) default_property: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeEnum {
    pub(crate) name: String,
    pub(crate) members: std::collections::HashMap<String, i64>,
}

impl From<&TypeDecl> for RuntimeType {
    fn from(value: &TypeDecl) -> Self {
        Self {
            name: value.name.clone(),
            is_structure: value.kind == TypeKind::Structure,
            fields: value
                .fields
                .iter()
                .map(|field| RuntimeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    array: field.array.clone(),
                    initializer: field.initializer.clone(),
                    with_events: false,
                })
                .collect(),
            subs: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Sub(method) => {
                        Some((key(&method.procedure.name), method.procedure.clone()))
                    }
                    _ => None,
                })
                .collect(),
            functions: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Function(method) => {
                        Some((key(&method.function.name), method.function.clone()))
                    }
                    ClassMember::Const(_) => None,
                    _ => None,
                })
                .collect(),
            properties: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Property(property) => Some(property),
                    _ => None,
                })
                .fold(std::collections::HashMap::new(), |mut props, property| {
                    let entry = props.entry(key(&property.name)).or_insert(RuntimeProperty {
                        get: None,
                        let_: None,
                        set: None,
                    });
                    let accessor = RuntimePropertyAccessor::from(property);
                    match property.kind {
                        crate::PropertyKind::Get => entry.get = Some(accessor),
                        crate::PropertyKind::Let => entry.let_ = Some(accessor),
                        crate::PropertyKind::Set => entry.set = Some(accessor),
                    }
                    props
                }),
            default_property: value.members.iter().find_map(|member| match member {
                ClassMember::Property(property) if property.is_default => {
                    Some(property.name.clone())
                }
                _ => None,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeField {
    pub(crate) name: String,
    pub(crate) ty: TypeName,
    pub(crate) array: Option<crate::ArrayDecl>,
    pub(crate) initializer: Option<crate::Expr>,
    pub(crate) with_events: bool,
}
