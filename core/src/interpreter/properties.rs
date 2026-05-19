use crate::runtime::{Diagnostic, Span, TypeName, Value};
use crate::{ClassProperty, PropertyKind, Stmt};

use super::objects::ensure_object;
use super::values::{coerce_assignment, key};
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn call_property_get(
        &mut self,
        object: Value,
        property: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
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
        let accessor = class
            .properties
            .get(&key(property))
            .and_then(|property| property.get.clone())
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Property '{}' has no Get accessor", property),
                    Some(span),
                )
            })?;
        let mut frame = Frame::default();
        // Property frames see module-level state like Sub and Function calls.
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, accessor.name));
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Return(value) => coerce_assignment(
                accessor.return_type.as_ref().expect("get return type"),
                value,
                span,
            ),
            ControlFlow::Continue => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                format!("Property Get '{}' must return a value", accessor.name),
                Some(accessor.span),
            )),
            ControlFlow::ExitSub => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Sub is only valid inside Sub",
                Some(accessor.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(accessor.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit statement escaped its block",
                Some(span),
            )),
        }
    }

    pub(crate) fn call_property_set(
        &mut self,
        object: Value,
        property: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
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
        let property_sig = class.properties.get(&key(property)).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Class '{}' has no field or property '{}'",
                    class.name, property
                ),
                Some(span),
            )
        })?;
        let accessor = if matches!(value, Value::Object(_) | Value::Nothing) {
            property_sig.set.as_ref().or(property_sig.let_.as_ref())
        } else {
            property_sig.let_.as_ref()
        }
        .cloned()
        .ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Property '{}' has no Let or Set accessor", property),
                Some(span),
            )
        })?;
        let Some(param) = accessor.params.first() else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Property {:?} '{}' expects one parameter",
                    accessor.kind, property
                ),
                Some(accessor.span),
            ));
        };
        let mut frame = Frame::default();
        frame.declare_object_alias("me", &class.name, instance, span)?;
        frame.declare(
            &param.name,
            param.ty.clone(),
            None,
            self.option_base,
            param.span,
            &self.types,
            &self.enums,
        )?;
        frame.assign(&param.name, value, span)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, accessor.name));
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function or Property Get",
                Some(accessor.span),
            )),
            ControlFlow::ExitSub => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Sub is only valid inside Sub",
                Some(accessor.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(accessor.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit statement escaped its block",
                Some(span),
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeProperty {
    pub(crate) get: Option<RuntimePropertyAccessor>,
    pub(crate) let_: Option<RuntimePropertyAccessor>,
    pub(crate) set: Option<RuntimePropertyAccessor>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimePropertyAccessor {
    pub(crate) name: String,
    pub(crate) kind: PropertyKind,
    pub(crate) params: Vec<crate::Parameter>,
    pub(crate) return_type: Option<TypeName>,
    pub(crate) body: Vec<Stmt>,
    pub(crate) span: Span,
}

impl From<&ClassProperty> for RuntimePropertyAccessor {
    fn from(value: &ClassProperty) -> Self {
        Self {
            name: value.name.clone(),
            kind: value.kind,
            params: value.params.clone(),
            return_type: value.return_type.clone(),
            body: value.body.clone(),
            span: value.span,
        }
    }
}
