use crate::runtime::well_known;
use crate::runtime::{ArrayValue, Diagnostic, Span, TypeName, Value, coerce_assignment};
use crate::{ClassProperty, Expr, PropertyKind, Stmt};
use std::rc::Rc;

use super::frame::Variable;
use super::interpreter::ScopeName;
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
            .and_then(|property| property.getter())
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
            well_known::SELF_KEY,
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
                accessor.span,
                self,
            )?;
        }
        self.scope_stack.push(ScopeName::Text(format!(
            "{}.{}",
            structure.name, accessor.name
        )));
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
            ControlFlow::Continue | ControlFlow::ExitProperty => {
                if accessor.is_iterator {
                    let elements = frame.take_yielded_values().unwrap_or_default();
                    let len = elements.len() as i64;
                    Ok(Value::Array(Rc::new(ArrayValue {
                        element_type: return_type,
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: 0,
                            upper: len - 1,
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
            | ControlFlow::ContinueFor
            | ControlFlow::ContinueWhile
            | ControlFlow::ContinueDo
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
        let accessor = property_sig.writer().cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Property '{}' has no Set accessor", property),
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
            well_known::SELF_KEY,
            TypeName::User(structure.name.clone()),
            variable,
            span,
            &self.types,
            &self.interfaces,
        )?;
        frame.declare(&param.name, param.ty.clone(), None, param.span, self)?;
        let _ = frame.assign(&param.name, value, span)?;
        self.scope_stack.push(ScopeName::Text(format!(
            "{}.{}",
            structure.name, accessor.name
        )));
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitProperty => Ok(()),
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
        let candidates = class
            .properties
            .get(&key(property))
            .map(|entry| entry.get.as_slice())
            .filter(|accessors| !accessors.is_empty())
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                    format!("Property '{}' has no Get accessor", property),
                    Some(span),
                )
            })?;
        // Overloaded getters are picked the way an overloaded method is:
        // `Item(1)` and `Item("a")` can reach different ones.
        let accessor = self
            .pick_overload(
                "Property Get",
                property,
                candidates,
                |accessor: &Rc<RuntimePropertyAccessor>| &accessor.params,
                || self.argument_types_of(args, caller_frame),
                span,
            )?
            .clone();
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&class.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        // Property frames see module-level state like Sub and Function calls.
        frame.declare_object_alias(well_known::SELF_KEY, &class.name, instance, span)?;
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
                accessor.span,
                self,
            )?;
        }
        self.scope_stack
            .push(ScopeName::Text(format!("{}.{}", class.name, accessor.name)));
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
            ControlFlow::Continue | ControlFlow::ExitProperty => {
                if accessor.is_iterator {
                    let elements = frame.take_yielded_values().unwrap_or_default();
                    let len = elements.len() as i64;
                    Ok(Value::Array(Rc::new(ArrayValue {
                        element_type: return_type,
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: 0,
                            upper: len - 1,
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
            | ControlFlow::ContinueFor
            | ControlFlow::ContinueWhile
            | ControlFlow::ContinueDo
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

    pub(crate) fn call_property_accessor(
        &mut self,
        accessor: Rc<RuntimePropertyAccessor>,
        args: &[Value],
        me: Option<Value>,
        class_context: Option<String>,
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.call_stack
            .push(ScopeName::Text(format!("Property {}", accessor.name)));
        self.scope_stack
            .push(ScopeName::Text(format!("Property {}", accessor.name)));
        let result = (|| {
            let mut frame = Frame::default();
            frame.inherit_modules_from(caller_frame)?;
            if let Some(ctx) = class_context {
                frame.set_class_context(ctx);
            }
            if let Some(me_val) = me {
                let class_name = match &me_val {
                    Value::Object(obj) => obj.borrow().class_name.clone(),
                    _ => "Object".to_string(), // fallback for primitives
                };
                if frame.class_context().is_none() {
                    frame.set_class_context(class_name.clone());
                }
                if let Value::Object(obj_rc) = me_val {
                    frame.declare_object_alias(well_known::SELF_KEY, &class_name, obj_rc, span)?;
                }
            }
            self.bind_parameter_values(&accessor.params, args, &mut frame, span)?;
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
                    accessor.span,
                    self,
                )?;
            }
            if accessor.is_iterator {
                frame.set_yield_mode();
            }
            let result = self.exec_block(&accessor.body, &mut frame);
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
                        Ok(Value::Array(Rc::new(crate::runtime::ArrayValue {
                            element_type: return_type,
                            elements,
                            bounds: vec![crate::runtime::ArrayBound {
                                lower: 0,
                                upper: len - 1,
                            }],
                            allocated: true,
                            dynamic: true,
                        })))
                    } else {
                        frame.get(&accessor.name, accessor.span)
                    }
                }
                _ => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit statement escaped its block",
                    Some(span),
                )),
            }
        })();
        self.scope_stack.pop();
        self.call_stack.pop();
        result
    }

    pub(crate) fn call_property_accessor_sub(
        &mut self,
        accessor: Rc<RuntimePropertyAccessor>,
        args: &[Value],
        me: Option<Value>,
        class_context: Option<String>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.call_stack
            .push(ScopeName::Text(format!("Property {}", accessor.name)));
        self.scope_stack
            .push(ScopeName::Text(format!("Property {}", accessor.name)));
        let result = (|| {
            let mut frame = Frame::default();
            if let Some(ctx) = class_context {
                frame.set_class_context(ctx);
            }
            if let Some(me_val) = me {
                let class_name = match &me_val {
                    Value::Object(obj) => obj.borrow().class_name.clone(),
                    _ => "Object".to_string(),
                };
                if frame.class_context().is_none() {
                    frame.set_class_context(class_name.clone());
                }
                if let Value::Object(obj_rc) = me_val {
                    frame.declare_object_alias(well_known::SELF_KEY, &class_name, obj_rc, span)?;
                }
            }
            self.bind_parameter_values(&accessor.params, args, &mut frame, span)?;
            let result = self.exec_block(&accessor.body, &mut frame);
            match result? {
                ControlFlow::Continue => Ok(()),
                _ => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit statement escaped its block",
                    Some(span),
                )),
            }
        })();
        self.scope_stack.pop();
        self.call_stack.pop();
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
        let candidates = property_sig.writers();
        if candidates.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Property '{}' has no Set accessor", property),
                Some(span),
            ));
        }
        // An indexed property is written `Item(2) = "two"`, so the indices come
        // first and the value last. That whole shape picks between overloads.
        let accessor = self
            .pick_overload(
                "Property Set",
                property,
                candidates,
                |accessor: &Rc<RuntimePropertyAccessor>| &accessor.params,
                || Self::argument_types_of_values(values),
                span,
            )?
            .clone();
        let mut frame = Frame::default();
        frame.declare_object_alias(well_known::SELF_KEY, &class.name, instance, span)?;
        self.bind_parameter_values(&accessor.params, values, &mut frame, span)?;
        self.scope_stack
            .push(ScopeName::Text(format!("{}.{}", class.name, accessor.name)));
        let result = self.exec_block(&accessor.body, &mut frame);
        self.scope_stack.pop();
        let result = match result? {
            ControlFlow::Continue | ControlFlow::ExitProperty => Ok(()),
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
            | ControlFlow::ContinueFor
            | ControlFlow::ContinueWhile
            | ControlFlow::ContinueDo
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
    /// The accessors of each kind, in declaration order.
    ///
    /// A property usually has one of each. Several are overloads, picked by
    /// the arguments at the use site the way a method call is.
    pub(crate) get: Vec<Rc<RuntimePropertyAccessor>>,
    pub(crate) set: Vec<Rc<RuntimePropertyAccessor>>,
}

impl RuntimeProperty {
    /// The getter to run when the use site offers nothing to choose by.
    pub(crate) fn getter(&self) -> Option<&Rc<RuntimePropertyAccessor>> {
        self.get.first()
    }

    /// The accessor used for a property assignment.
    pub(crate) fn writer(&self) -> Option<&Rc<RuntimePropertyAccessor>> {
        self.set.first()
    }

    /// Every accessor a write could go through.
    pub(crate) fn writers(&self) -> &[Rc<RuntimePropertyAccessor>] {
        &self.set
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimePropertyAccessor {
    pub(crate) name: String,
    pub(crate) kind: PropertyKind,
    /// True when the property belongs to the class rather than to an instance.
    ///
    /// Resolution needs this to tell `Shared` access apart from reading a
    /// property off another instance: only the former runs without a receiver.
    pub(crate) is_shared: bool,
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
            is_shared: value.is_shared,
            is_iterator: value.is_iterator,
            params: value.params.clone(),
            return_type: value.return_type.clone(),
            body: value.body.clone(),
            span: value.span,
        }
    }
}
