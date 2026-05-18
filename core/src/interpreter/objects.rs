use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{Diagnostic, ObjectValue, Span, Value};
use crate::{ClassMember, Expr, ExprKind, Function, Procedure, PropertyKind};

use super::arrays::array_element_mut;
use super::frame::Variable;
use super::properties::{RuntimeProperty, RuntimePropertyAccessor};
use super::records::{RuntimeField, read_field_member, write_member};
use super::values::{default_value, key};
use super::{Frame, Interpreter};

impl Interpreter {
    pub(crate) fn new_object(
        &mut self,
        class_name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let class = self.classes.get(&key(class_name)).cloned().ok_or_else(|| {
            Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
        })?;
        let mut fields = HashMap::new();
        for field in &class.fields {
            fields.insert(
                key(&field.name),
                default_value(&field.ty, &self.types, &self.enums, span)?,
            );
        }
        let object = Value::Object(Rc::new(RefCell::new(ObjectValue {
            class_name: class.name.clone(),
            fields,
        })));
        if let Some(init) = class.subs.get("initialize") {
            self.call_method_sub(object.clone(), &init.name, args, caller_frame, span)?;
        } else if !args.is_empty() {
            return Err(Diagnostic::new(
                format!("Class '{}' has no Initialize constructor", class.name),
                Some(span),
            ));
        }
        Ok(object)
    }

    pub(crate) fn read_member(
        &mut self,
        value: &Value,
        member: &str,
        _frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if object_has_field(value, member) {
            return read_field_member(value, member, span);
        }
        if matches!(value, Value::Nothing) {
            return Err(Diagnostic::new("Object reference is Nothing", Some(span))
                .with_primary_label("attempted to access a member on Nothing")
                .with_help("assign an object before accessing its members"));
        }
        if matches!(value, Value::Object(_)) {
            return self.call_property_get(value.clone(), member, span);
        }
        read_field_member(value, member, span)
    }

    pub(crate) fn assign_member(
        &mut self,
        target: &Expr,
        member: &str,
        value: Value,
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match &target.kind {
            ExprKind::Variable(name) => {
                let variable = frame.variable(name, target.span)?;
                self.assign_member_to_variable(variable, member, value, span)
            }
            ExprKind::Me => {
                let variable = frame.variable("me", target.span)?;
                self.assign_member_to_variable(variable, member, value, span)
            }
            ExprKind::Call { name, args } => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "Array access requires exactly one index",
                        Some(target.span),
                    ));
                }
                let index = frame.simple_index_value(&args[0], span)?;
                let variable = frame.variable(name, target.span)?;
                let mut root = variable.cell.borrow_mut();
                let element = array_element_mut(&mut root, index, span)?;
                if object_has_field(element, member) || !matches!(element, Value::Object(_)) {
                    return write_member(element, member, value, span);
                }
                let object = element.clone();
                drop(root);
                self.call_property_set(object, member, value, span)
            }
            ExprKind::MemberAccess { .. } | ExprKind::MemberCall { .. } | ExprKind::New { .. } => {
                let target_value = self.eval_expr(target, frame)?;
                self.assign_member_to_value(target_value, member, value, span)
            }
            _ => {
                let target_value = self.eval_expr(target, frame)?;
                self.assign_member_to_value(target_value, member, value, span)
            }
        }
    }

    pub(crate) fn assign_member_to_variable(
        &mut self,
        variable: Variable,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut root = variable.cell.borrow_mut();
        if object_has_field(&root, member) || !matches!(&*root, Value::Object(_)) {
            return write_member(&mut root, member, value, span);
        }
        let object = root.clone();
        drop(root);
        self.call_property_set(object, member, value, span)
    }

    pub(crate) fn assign_member_to_value(
        &mut self,
        mut target: Value,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if object_has_field(&target, member) || !matches!(target, Value::Object(_)) {
            return write_member(&mut target, member, value, span);
        }
        self.call_property_set(target, member, value, span)
    }
}

pub(crate) fn object_has_field(value: &Value, field: &str) -> bool {
    if let Value::Object(object) = value {
        return object.borrow().fields.contains_key(&key(field));
    }
    false
}

pub(crate) fn ensure_object(
    value: Value,
    span: Span,
) -> Result<Rc<RefCell<ObjectValue>>, Diagnostic> {
    match value {
        Value::Object(object) => Ok(object),
        Value::Nothing => Err(Diagnostic::new("Object reference is Nothing", Some(span))
            .with_primary_label("attempted to call a method on Nothing")
            .with_help("assign an object before calling its methods")),
        _ => Err(Diagnostic::new(
            "Method call requires an object",
            Some(span),
        )),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeClass {
    pub(crate) name: String,
    pub(crate) fields: Vec<RuntimeField>,
    pub(crate) subs: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) properties: HashMap<String, RuntimeProperty>,
}

impl From<&crate::ClassDecl> for RuntimeClass {
    fn from(value: &crate::ClassDecl) -> Self {
        let mut fields = Vec::new();
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut properties = HashMap::new();
        for member in &value.members {
            match member {
                ClassMember::Field(field) => fields.push(RuntimeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                }),
                ClassMember::Sub(method) => {
                    subs.insert(key(&method.procedure.name), method.procedure.clone());
                }
                ClassMember::Function(method) => {
                    functions.insert(key(&method.function.name), method.function.clone());
                }
                ClassMember::Property(property) => {
                    let property_entry =
                        properties
                            .entry(key(&property.name))
                            .or_insert_with(|| RuntimeProperty {
                                get: None,
                                let_: None,
                                set: None,
                            });
                    let accessor = RuntimePropertyAccessor::from(property);
                    match property.kind {
                        PropertyKind::Get => property_entry.get = Some(accessor),
                        PropertyKind::Let => property_entry.let_ = Some(accessor),
                        PropertyKind::Set => property_entry.set = Some(accessor),
                    }
                }
            }
        }
        Self {
            name: value.name.clone(),
            fields,
            subs,
            functions,
            properties,
        }
    }
}
