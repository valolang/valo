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
    let record = match value {
        Value::Record(v) => v,
        Value::BoxedRecord(v, _) => v,
        _ => {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Member access requires a user-defined Type value",
                Some(span),
            ));
        }
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
    if let Value::ComObject(com_obj) = value {
        let old_val = crate::runtime::com::invoke_com(
            com_obj,
            field,
            &[],
            2, // DISPATCH_PROPERTYGET
            span,
        )
        .unwrap_or(Value::Empty);
        crate::runtime::com::invoke_com(
            com_obj,
            field,
            &[new_value],
            4, // DISPATCH_PROPERTYPUT
            span,
        )?;
        return Ok(old_val);
    }
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

    let (mut record, interface) = match value {
        Value::Record(v) => (v.as_ref().clone(), None),
        Value::BoxedRecord(v, interface) => (v.as_ref().clone(), Some(interface.clone())),
        _ => {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Member assignment requires a user-defined Type value",
                Some(span),
            ));
        }
    };

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

    *value = match interface {
        Some(interface) => Value::BoxedRecord(Rc::new(record), interface),
        None => Value::Record(Rc::new(record)),
    };

    Ok(old)
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeType {
    pub(crate) name: String,
    pub(crate) type_params: Vec<String>,
    pub(crate) is_structure: bool,
    pub(crate) implements: Vec<TypeName>,
    pub(crate) fields: Vec<RuntimeField>,
    pub(crate) subs: std::collections::HashMap<String, crate::Procedure>,
    pub(crate) functions: std::collections::HashMap<String, crate::Function>,
    pub(crate) properties: std::collections::HashMap<String, RuntimeProperty>,
    pub(crate) operators: std::collections::HashMap<crate::OperatorKind, crate::Function>,
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
            type_params: value.type_params.clone(),
            is_structure: value.kind == TypeKind::Structure,
            implements: value.implements.clone(),
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
                .chain(value.members.iter().flat_map(|member| {
                    match member {
                        ClassMember::Field(field) => vec![RuntimeField {
                            name: field.name.clone(),
                            ty: field.ty.clone().unwrap_or(TypeName::Variant),
                            array: field.array.clone(),
                            initializer: field.initializer.clone(),
                            with_events: field.with_events,
                        }],
                        ClassMember::Fields(fs) => fs
                            .iter()
                            .map(|field| RuntimeField {
                                name: field.name.clone(),
                                ty: field.ty.clone().unwrap_or(TypeName::Variant),
                                array: field.array.clone(),
                                initializer: field.initializer.clone(),
                                with_events: field.with_events,
                            })
                            .collect(),
                        _ => Vec::new(),
                    }
                }))
                .collect(),
            subs: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Sub(method) => {
                        Some((key(&method.procedure.name), method.procedure.clone()))
                    }
                    ClassMember::Field(_)
                    | ClassMember::Fields(_)
                    | ClassMember::Const(_)
                    | ClassMember::Event(_)
                    | ClassMember::Function(_)
                    | ClassMember::Iterator(_)
                    | ClassMember::Property(_)
                    | ClassMember::Type(_)
                    | ClassMember::Declare(_)
                    | ClassMember::Enum(_)
                    | ClassMember::Operator(_)
                    | ClassMember::Class(_) => None,
                })
                .collect(),
            functions: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Function(method) => {
                        Some((key(&method.function.name), method.function.clone()))
                    }
                    ClassMember::Field(_)
                    | ClassMember::Fields(_)
                    | ClassMember::Const(_)
                    | ClassMember::Event(_)
                    | ClassMember::Sub(_)
                    | ClassMember::Iterator(_)
                    | ClassMember::Property(_)
                    | ClassMember::Type(_)
                    | ClassMember::Declare(_)
                    | ClassMember::Enum(_)
                    | ClassMember::Operator(_)
                    | ClassMember::Class(_) => None,
                })
                .collect(),
            properties: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Property(property) => Some(property),
                    ClassMember::Field(_)
                    | ClassMember::Fields(_)
                    | ClassMember::Const(_)
                    | ClassMember::Event(_)
                    | ClassMember::Sub(_)
                    | ClassMember::Function(_)
                    | ClassMember::Iterator(_)
                    | ClassMember::Type(_)
                    | ClassMember::Declare(_)
                    | ClassMember::Enum(_)
                    | ClassMember::Operator(_)
                    | ClassMember::Class(_) => None,
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
            operators: value
                .members
                .iter()
                .filter_map(|member| match member {
                    ClassMember::Operator(op) => Some((
                        op.kind,
                        crate::Function {
                            visibility: op.visibility,
                            name: format!("{:?}", op.kind),
                            is_iterator: false,
                            type_params: Vec::new(),
                            generic_constraints: Vec::new(),
                            params: op.params.clone(),
                            return_type: op.return_type.clone(),
                            return_slot: None,
                            body: op.body.clone(),
                            span: op.span,
                        },
                    )),
                    _ => None,
                })
                .collect(),
            default_property: value.members.iter().find_map(|member| match member {
                ClassMember::Property(property) if property.is_default => {
                    Some(property.name.clone())
                }
                _ => None,
            }),
        }
    }
}

impl RuntimeType {
    pub(crate) fn generic_display_name(&self) -> String {
        if self.type_params.is_empty() {
            self.name.clone()
        } else {
            format!("{}(Of {})", self.name, self.type_params.join(", "))
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

#[derive(Debug, Clone)]
pub(crate) struct RuntimeInterface {
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) type_params: Vec<String>,
    #[allow(dead_code)]
    pub(crate) subs: std::collections::HashMap<String, crate::InterfaceMethod>,
    #[allow(dead_code)]
    pub(crate) functions: std::collections::HashMap<String, crate::InterfaceMethod>,
    #[allow(dead_code)]
    pub(crate) properties: std::collections::HashMap<String, crate::InterfaceProperty>,
}

impl From<&crate::InterfaceDecl> for RuntimeInterface {
    fn from(value: &crate::InterfaceDecl) -> Self {
        Self {
            name: value.name.clone(),
            type_params: value.type_params.clone(),
            subs: value
                .members
                .iter()
                .filter_map(|member| match member {
                    crate::InterfaceMember::Sub(method) => {
                        Some((key(&method.name), method.clone()))
                    }
                    _ => None,
                })
                .collect(),
            functions: value
                .members
                .iter()
                .filter_map(|member| match member {
                    crate::InterfaceMember::Function(method) => {
                        Some((key(&method.name), method.clone()))
                    }
                    _ => None,
                })
                .collect(),
            properties: value
                .members
                .iter()
                .filter_map(|member| match member {
                    crate::InterfaceMember::Property(property) => {
                        Some((key(&property.name), property.clone()))
                    }
                    _ => None,
                })
                .collect(),
        }
    }
}
