use crate::runtime::{ArrayValue, Diagnostic, Span, TypeName, Value, coerce_assignment};
use crate::{ClassProperty, Expr, PropertyKind, Stmt};
use std::rc::Rc;

use super::frame::Variable;
use super::objects::ensure_object;
use super::values::key;
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn call_record_property_get(
        &mut self,
        record_val: Value,
        property: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let (type_name, record_val) = match &record_val {
            Value::Record(record) => (record.type_name.clone(), record_val),
            Value::BoxedRecord(record, _) => {
                (record.type_name.clone(), Value::Record(record.clone()))
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Property access requires a Structure value",
                    Some(span),
                ));
            }
        };
        let structure = self.types.get(&key(&type_name)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Structure '{}' is not defined", type_name),
                Some(span),
            )
        })?;
        let accessor = structure
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
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&structure.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        frame.declare_const(
            "me",
            TypeName::User(structure.name.clone()),
            record_val.clone(),
            span,
        )?;
        self.bind_parameters(&accessor.params, args, caller_frame, &mut frame)?;
        let return_type = self.resolve_type_name(
            accessor.return_type.as_ref().expect("get return type"),
            &frame,
            span,
        )?;
        if !frame.has_variable(&accessor.name) {
            frame.declare(
                &accessor.name,
                return_type.clone(),
                None,
                self.option_base,
                accessor.span,
                self,
            )?;
        }
        self.scope_stack
            .push(format!("{}.{}", structure.name, accessor.name));
        if accessor.is_iterator {
            frame.set_yield_mode();
        }
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Return(value) => {
                if accessor.is_iterator {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::CONTROL_FLOW,
                        "Return is not allowed inside Iterator; use Yield or Exit Function",
                        Some(accessor.span),
                    ));
                }
                coerce_assignment(&return_type, value, span)
            }
            ControlFlow::Continue => {
                if accessor.is_iterator {
                    let elements = frame.take_yielded_values().unwrap_or_default();
                    let len = elements.len() as i64;
                    Ok(Value::Array(Rc::new(ArrayValue {
                        element_type: return_type,
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: self.option_base,
                            upper: self.option_base + len - 1,
                        }],
                        allocated: true,
                        dynamic: true,
                    })))
                } else {
                    frame.get(&accessor.name, accessor.span)
                }
            }
            ControlFlow::Terminate => Ok(Value::Empty),
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

    pub(crate) fn call_record_property_set(
        &mut self,
        variable: Variable,
        property: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let record_val = variable.borrow().clone();
        let type_name = match &record_val {
            Value::Record(record) => record.type_name.clone(),
            Value::BoxedRecord(record, _) => record.type_name.clone(),
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Property assignment requires a Structure value",
                    Some(span),
                ));
            }
        };
        let structure = self.types.get(&key(&type_name)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Structure '{}' is not defined", type_name),
                Some(span),
            )
        })?;
        let property_sig = structure.properties.get(&key(property)).ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!(
                    "Structure '{}' has no field or property '{}'",
                    structure.name, property
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
        frame.declare_alias(
            "me",
            TypeName::User(structure.name.clone()),
            variable,
            span,
            &self.types,
            &self.interfaces,
        )?;
        frame.declare(
            &param.name,
            param.ty.clone(),
            None,
            self.option_base,
            param.span,
            self,
        )?;
        let _ = frame.assign(&param.name, value, span)?;
        self.scope_stack
            .push(format!("{}.{}", structure.name, accessor.name));
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Terminate => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function or Property Get",
                Some(accessor.span),
            )),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit statement escaped its block",
                Some(span),
            )),
        }
    }

    pub(crate) fn call_property_get(
        &mut self,
        object: Value,
        property: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if let Value::ComObject(ref com_obj) = object {
            let mut eval_args = Vec::with_capacity(args.len());
            for arg in args {
                eval_args.push(self.eval_expr(arg, caller_frame)?);
            }
            return crate::runtime::com::invoke_com(
                com_obj, property, &eval_args, 3, // DISPATCH_METHOD | DISPATCH_PROPERTYGET
                span,
            );
        }

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
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&class.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        // Property frames see module-level state like Sub and Function calls.
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameters(&accessor.params, args, caller_frame, &mut frame)?;

        let return_type = accessor
            .return_type
            .as_ref()
            .expect("get return type")
            .clone();
        let return_type = self.resolve_type_name(&return_type, &frame, span)?;

        if !frame.has_variable(&accessor.name) {
            frame.declare(
                &accessor.name,
                return_type.clone(),
                None,
                self.option_base,
                accessor.span,
                self,
            )?;
        }
        self.scope_stack
            .push(format!("{}.{}", class.name, accessor.name));
        if accessor.is_iterator {
            frame.set_yield_mode();
        }
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        let result = match result? {
            ControlFlow::Return(value) => {
                if accessor.is_iterator {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::CONTROL_FLOW,
                        "Return is not allowed inside Iterator; use Yield or Exit Function",
                        Some(accessor.span),
                    ));
                }
                coerce_assignment(&return_type, value, span)
            }
            ControlFlow::Continue => {
                if accessor.is_iterator {
                    let elements = frame.take_yielded_values().unwrap_or_default();
                    let len = elements.len() as i64;
                    Ok(Value::Array(Rc::new(ArrayValue {
                        element_type: return_type,
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: self.option_base,
                            upper: self.option_base + len - 1,
                        }],
                        allocated: true,
                        dynamic: true,
                    })))
                } else {
                    frame.get(&accessor.name, accessor.span)
                }
            }
            ControlFlow::Terminate => Ok(Value::Empty),
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
        };
        self.terminate_frame_variables(frame, span)?;
        result
    }

    pub(crate) fn call_property_set(
        &mut self,
        object: Value,
        property: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.call_property_set_values(object, property, &[value], span)
    }

    pub(crate) fn call_property_set_values(
        &mut self,
        object: Value,
        property: &str,
        values: &[Value],
        span: Span,
    ) -> Result<(), Diagnostic> {
        if let Value::ComObject(ref com_obj) = object {
            crate::runtime::com::invoke_com(
                com_obj, property, values, 4, // DISPATCH_PROPERTYPUT
                span,
            )?;
            return Ok(());
        }

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
        let value = values.last().cloned().unwrap_or(Value::Missing);
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
        let mut frame = Frame::default();
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameter_values(&accessor.params, values, &mut frame, span)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, accessor.name));
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        let result = match result? {
            ControlFlow::Continue => Ok(()),
            ControlFlow::Terminate => Ok(()),
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
        };
        self.terminate_frame_variables(frame, span)?;
        result
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
    pub(crate) is_iterator: bool,
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
            is_iterator: value.is_iterator,
            params: value.params.clone(),
            return_type: value.return_type.clone(),
            body: value.body.clone(),
            span: value.span,
        }
    }
}
