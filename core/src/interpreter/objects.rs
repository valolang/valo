use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{Diagnostic, EventBinding, ObjectValue, Span, Value};
use crate::{ClassMember, Expr, ExprKind, Function, Procedure, PropertyKind};

use super::arrays::array_element_mut;
use super::frame::Variable;
use super::properties::{RuntimeProperty, RuntimePropertyAccessor};
use super::records::{RuntimeField, read_field_member, write_member};
use super::values::{default_value, key};
use super::{Frame, Interpreter};

impl Interpreter {
    pub(crate) fn raise_event(
        &mut self,
        name: &str,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let source = frame.get("me", span)?;
        let Value::Object(source) = source else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "RaiseEvent is only valid inside class methods",
                Some(span),
            ));
        };
        let class_name = source.borrow().class_name.clone();
        let class = self
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
        if !class.events.contains_key(&key(name)) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("Class '{}' has no event '{}'", class.name, name),
                Some(span),
            ));
        }
        let mut values = Vec::new();
        for arg in args {
            values.push(self.eval_expr(arg, frame)?);
        }
        let bindings = source.borrow().event_bindings.clone();
        for binding in bindings {
            if binding.event_name.eq_ignore_ascii_case(name) {
                self.call_method_sub_values(
                    Value::Object(binding.target.clone()),
                    &binding.handler_name,
                    &values,
                    frame,
                    span,
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn new_object(
        &mut self,
        class_name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let class_name = self.resolve_user_type_name(class_name, caller_frame, span)?;
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS,
                    format!("'{}' is not a class", class_name),
                    Some(span),
                )
            })?;
        let mut fields = HashMap::new();
        for field in &class.fields {
            let field_ty = self.resolve_type_name(&field.ty, caller_frame, span)?;
            fields.insert(
                key(&field.name),
                default_value(&field_ty, &self.types, &self.enums, span)?,
            );
        }
        let object = Value::Object(Rc::new(RefCell::new(ObjectValue {
            class_name: class.name.clone(),
            fields,
            event_bindings: Vec::new(),
            terminated: false,
        })));
        if let Some(init) = class
            .subs
            .get("initialize")
            .or_else(|| class.subs.get("class_initialize"))
        {
            self.call_method_sub(object.clone(), &init.name, args, caller_frame, span)?;
        } else if !args.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
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
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if object_has_field(value, member) {
            return read_field_member(value, member, span);
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
        if matches!(value, Value::Object(_)) {
            return self.call_property_get(value.clone(), member, &[], frame, span);
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
                if let Ok(module_key) = self.resolve_module_qualifier(name, frame, span) {
                    if frame.module_key() != Some(module_key.as_str())
                        && !self
                            .public_values
                            .get(&module_key)
                            .is_some_and(|values| values.contains(&key(member)))
                    {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                            format!("Module member '{}.{}' is Private", name, member),
                            Some(span),
                        ));
                    }
                    let module_frame =
                        self.module_frames.get_mut(&module_key).ok_or_else(|| {
                            Diagnostic::new(
                                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                                format!("Module '{}' is not loaded", name),
                                Some(span),
                            )
                        })?;
                    let old = module_frame.assign(member, value, span)?;
                    return self.maybe_terminate(old, span);
                }
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
                        crate::runtime::DiagnosticCode::ARRAY,
                        "Array access requires exactly one index",
                        Some(target.span),
                    ));
                }
                let index = frame.simple_index_value(&args[0], span)?;
                let variable = frame.variable(name, target.span)?;
                let mut root = variable.cell.borrow_mut();
                let element = array_element_mut(&mut root, index, span)?;
                if object_has_field(element, member) || !matches!(element, Value::Object(_)) {
                    let old = write_member(element, member, value, span)?;
                    self.maybe_terminate(old, span)?;
                    return Ok(());
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
            return self.write_object_member(&mut root, member, value, span);
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
            return self.write_object_member(&mut target, member, value, span);
        }
        self.call_property_set(target, member, value, span)
    }

    pub(crate) fn assign_bare_class_field(
        &mut self,
        owner: Value,
        field: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut owner_value = owner;
        self.write_object_member(&mut owner_value, field, value, span)
    }

    fn write_object_member(
        &mut self,
        target: &mut Value,
        member: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let owner = match target {
            Value::Object(object) if object.borrow().fields.contains_key(&key(member)) => {
                Some(object.clone())
            }
            _ => None,
        };
        let old_value = write_member(target, member, value, span)?;
        if let Some(owner) = owner {
            let new_value = owner
                .borrow()
                .fields
                .get(&key(member))
                .cloned()
                .unwrap_or(Value::Nothing);
            self.rebind_withevents_field(owner, member, &old_value, &new_value);
        }
        self.maybe_terminate(old_value, span)?;
        Ok(())
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
        Value::Nothing => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            "Object reference is Nothing",
            Some(span),
        )
        .with_primary_label("attempted to call a method on Nothing")
        .with_help("assign an object before calling its methods")),
        _ => Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
            "Method call requires an object",
            Some(span),
        )),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeClass {
    pub(crate) name: String,
    pub(crate) fields: Vec<RuntimeField>,
    pub(crate) events: HashMap<String, RuntimeEvent>,
    pub(crate) subs: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) properties: HashMap<String, RuntimeProperty>,
    pub(crate) default_member: Option<String>,
}

impl From<&crate::ClassDecl> for RuntimeClass {
    fn from(value: &crate::ClassDecl) -> Self {
        let mut fields = Vec::new();
        let mut events = HashMap::new();
        let mut subs = HashMap::new();
        let mut functions = HashMap::new();
        let mut properties = HashMap::new();
        let mut default_member = None;
        for member in &value.members {
            match member {
                ClassMember::Field(field) => fields.push(RuntimeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    with_events: field.with_events,
                }),
                ClassMember::Event(event) => {
                    events.insert(
                        key(&event.name),
                        RuntimeEvent {
                            name: event.name.clone(),
                        },
                    );
                }
                ClassMember::Sub(method) => {
                    subs.insert(key(&method.procedure.name), method.procedure.clone());
                }
                ClassMember::Function(method) => {
                    functions.insert(key(&method.function.name), method.function.clone());
                }
                ClassMember::Property(property) => {
                    if property.is_default {
                        default_member = Some(property.name.clone());
                    }
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
            events,
            subs,
            functions,
            properties,
            default_member,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeEvent {
    pub(crate) name: String,
}

impl Interpreter {
    pub(crate) fn rebind_withevents_field(
        &mut self,
        owner: Rc<RefCell<ObjectValue>>,
        field: &str,
        old_value: &Value,
        value: &Value,
    ) {
        let owner_class_name = owner.borrow().class_name.clone();
        let Some(owner_class) = self.classes.get(&key(&owner_class_name)).cloned() else {
            return;
        };
        let Some(field_sig) = owner_class
            .fields
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(field) && candidate.with_events)
        else {
            return;
        };
        if let Value::Object(source) = old_value {
            source.borrow_mut().event_bindings.retain(|binding| {
                !(Rc::ptr_eq(&binding.target, &owner)
                    && binding
                        .handler_name
                        .to_ascii_lowercase()
                        .starts_with(&format!("{}_", field_sig.name.to_ascii_lowercase())))
            });
        }
        let Value::Object(source) = value else {
            return;
        };
        let source_class_name = source.borrow().class_name.clone();
        let Some(source_class) = self.classes.get(&key(&source_class_name)) else {
            return;
        };
        let mut bindings = Vec::new();
        for event in source_class.events.values() {
            let handler_name = format!("{}_{}", field_sig.name, event.name);
            if owner_class.subs.contains_key(&key(&handler_name)) {
                bindings.push(EventBinding {
                    event_name: event.name.clone(),
                    target: owner.clone(),
                    handler_name,
                });
            }
        }
        source.borrow_mut().event_bindings.extend(bindings);
    }
}
