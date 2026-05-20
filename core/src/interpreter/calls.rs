use crate::runtime::{Diagnostic, Span, TypeName, Value};
use crate::{Expr, ExprKind, Function, PassingMode};

use super::frame::Variable;
use super::objects::ensure_object;
use super::values::{coerce_assignment, key};
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn call_record_sub_variable(
        &mut self,
        variable: Variable,
        method: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let record = variable.cell.borrow().clone();
        let Value::Record { type_name, .. } = &record else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Structure method call requires a Structure value",
                Some(span),
            ));
        };
        let structure = self.types.get(&key(type_name)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Structure '{}' is not defined", type_name),
                Some(span),
            )
        })?;
        if method.eq_ignore_ascii_case("Constructor") {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                "Structure constructor cannot be called as a normal method",
                Some(span),
            ));
        }
        let procedure = structure.subs.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Structure '{}' has no method '{}'", structure.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&structure.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        frame.declare_alias("me", TypeName::User(structure.name.clone()), variable, span)?;
        self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;
        self.scope_stack
            .push(format!("{}.{}", structure.name, procedure.name));
        let result = self.exec_block(&procedure.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit statement escaped its block",
                Some(span),
            )),
        }
    }

    pub(crate) fn call_record_function(
        &mut self,
        record: Value,
        method: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let Value::Record { type_name, .. } = &record else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Structure method call requires a Structure value",
                Some(span),
            ));
        };
        let structure = self.types.get(&key(type_name)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Structure '{}' is not defined", type_name),
                Some(span),
            )
        })?;
        if let Some(function) = structure.functions.get(&key(method)).cloned() {
            let mut frame = Frame::default();
            frame.inherit_modules_from(caller_frame)?;
            if let Some((module_key, _)) = key(&structure.name).split_once('.') {
                frame.set_module_key(module_key.to_string());
            }
            frame.declare_const("me", TypeName::User(structure.name.clone()), record, span)?;
            self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;
            let return_type = self.resolve_type_name(&function.return_type, &frame, span)?;
            if !frame.has_variable(&function.name) {
                frame.declare(
                    &function.name,
                    return_type.clone(),
                    None,
                    self.option_base,
                    function.span,
                    &self.types,
                    &self.enums,
                )?;
            }
            self.scope_stack
                .push(format!("{}.{}", structure.name, function.name));
            if function.is_iterator {
                frame.set_yield_mode();
            }
            let result = self.exec_block(&function.body, &mut frame);
            self.scope_stack.pop();
            return match result? {
                ControlFlow::Return(value) => {
                    if function.is_iterator {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Return is not allowed inside Iterator; use Yield or Exit Function",
                            Some(function.span),
                        ));
                    }
                    coerce_assignment(&return_type, value, span)
                }
                ControlFlow::Continue | ControlFlow::ExitFunction => {
                    if function.is_iterator {
                        let elements = frame.take_yielded_values().unwrap_or_default();
                        let len = elements.len() as i64;
                        Ok(Value::Array {
                            element_type: function.return_type.clone(),
                            elements,
                            bounds: vec![crate::runtime::ArrayBound {
                                lower: self.option_base,
                                upper: self.option_base + len - 1,
                            }],
                            allocated: true,
                        })
                    } else {
                        frame.get(&function.name, function.span)
                    }
                }
                _ => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit statement escaped its block",
                    Some(span),
                )),
            };
        }
        if structure.properties.contains_key(&key(method)) {
            return self.call_record_property_get(record, method, args, caller_frame, span);
        }
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Structure '{}' has no method or property '{}'",
                structure.name, method
            ),
            Some(span),
        ))
    }

    pub(crate) fn call_function(
        &mut self,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let module_key = self.resolve_function_module(name, caller_frame, span)?;
        let lookup = qualified_key(module_key.as_deref(), name);
        let function = self.functions.get(&lookup).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Function '{}' is not defined", name),
                Some(span),
            )
        })?;

        self.call_stack
            .push(format!("Function '{}'", function.name));
        self.scope_stack.push(format!("Function {}", function.name));
        let result = (|| {
            let mut frame = Frame::default();
            if let Some(module_key) = &module_key {
                frame = self.module_frames.get(module_key).cloned().ok_or_else(|| {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Module '{}' is not loaded", module_key),
                        Some(span),
                    )
                })?;
                frame.set_module_key(module_key.clone());
            } else {
                frame.inherit_modules_from(caller_frame)?;
            }
            self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;
            let return_type = self.resolve_type_name(&function.return_type, &frame, span)?;
            if !frame.has_variable(&function.name) {
                frame.declare(
                    &function.name,
                    return_type.clone(),
                    None,
                    self.option_base,
                    function.span,
                    &self.types,
                    &self.enums,
                )?;
            }

            if function.is_iterator {
                frame.set_yield_mode();
            }

            let result = match self.exec_block(&function.body, &mut frame)? {
                ControlFlow::Return(value) => {
                    if function.is_iterator {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Return is not allowed inside Iterator; use Yield or Exit Function",
                            Some(function.span),
                        ));
                    }
                    coerce_assignment(&return_type, value, span)
                }
                ControlFlow::Continue | ControlFlow::ExitFunction => {
                    if function.is_iterator {
                        let elements = frame.take_yielded_values().unwrap_or_default();
                        let len = elements.len() as i64;
                        Ok(Value::Array {
                            element_type: function.return_type.clone(),
                            elements,
                            bounds: vec![crate::runtime::ArrayBound {
                                lower: self.option_base,
                                upper: self.option_base + len - 1,
                            }],
                            allocated: true,
                        })
                    } else {
                        frame.get(&function.name, function.span)
                    }
                }
                ControlFlow::ExitSub => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit Sub is only valid inside Sub",
                    Some(function.span),
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
        })();
        let result = result.map_err(|diagnostic| self.with_stack_context(diagnostic));
        self.scope_stack.pop();
        self.call_stack.pop();
        result
    }

    pub(crate) fn call_sub(
        &mut self,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let module_key = self.resolve_sub_module(name, caller_frame, span)?;
        let lookup = qualified_key(module_key.as_deref(), name);
        let procedure = self.procedures.get(&lookup).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Sub '{}' is not defined", name),
                Some(span),
            )
        })?;

        self.call_stack.push(format!("Sub '{}'", procedure.name));
        self.scope_stack.push(format!("Sub {}", procedure.name));
        let result = (|| {
            let mut frame = Frame::default();
            if let Some(module_key) = &module_key {
                frame = self.module_frames.get(module_key).cloned().ok_or_else(|| {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Module '{}' is not loaded", module_key),
                        Some(span),
                    )
                })?;
                frame.set_module_key(module_key.clone());
            } else {
                frame.inherit_modules_from(caller_frame)?;
            }
            self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;

            let result = match self.exec_block(&procedure.body, &mut frame)? {
                ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
                ControlFlow::Return(_) => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Return is only allowed inside Function",
                    Some(procedure.span),
                )),
                ControlFlow::ExitFunction => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit Function is only valid inside Function",
                    Some(procedure.span),
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
        })();
        let result = result.map_err(|diagnostic| self.with_stack_context(diagnostic));
        self.scope_stack.pop();
        self.call_stack.pop();
        result
    }

    pub(crate) fn call_module_function(
        &mut self,
        qualifier: &str,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let module_key = self.resolve_module_qualifier(qualifier, caller_frame, span)?;
        let function = self
            .functions
            .get(&qualified_key(Some(&module_key), name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Function '{}.{}' is not defined", qualifier, name),
                    Some(span),
                )
            })?;
        if caller_frame.module_key() != Some(module_key.as_str())
            && !crate::modules::is_public(function.visibility)
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                format!("Function '{}.{}' is Private", qualifier, name),
                Some(span),
            ));
        }
        let mut frame = self
            .module_frames
            .get(&module_key)
            .cloned()
            .expect("module loaded");
        frame.set_module_key(module_key);
        self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;
        let return_type = self.resolve_type_name(&function.return_type, &frame, span)?;
        if !frame.has_variable(&function.name) {
            frame.declare(
                &function.name,
                return_type.clone(),
                None,
                self.option_base,
                function.span,
                &self.types,
                &self.enums,
            )?;
        }
        if function.is_iterator {
            frame.set_yield_mode();
        }
        match self.exec_block(&function.body, &mut frame)? {
            ControlFlow::Return(value) => {
                if function.is_iterator {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::CONTROL_FLOW,
                        "Return is not allowed inside Iterator; use Yield or Exit Function",
                        Some(function.span),
                    ));
                }
                coerce_assignment(&return_type, value, span)
            }
            ControlFlow::Continue | ControlFlow::ExitFunction => {
                if function.is_iterator {
                    let elements = frame.take_yielded_values().unwrap_or_default();
                    let len = elements.len() as i64;
                    Ok(Value::Array {
                        element_type: function.return_type.clone(),
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: self.option_base,
                            upper: self.option_base + len - 1,
                        }],
                        allocated: true,
                    })
                } else {
                    frame.get(&function.name, function.span)
                }
            }

            ControlFlow::ExitSub => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Sub is only valid inside Sub",
                Some(function.span),
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

    pub(crate) fn call_module_sub(
        &mut self,
        qualifier: &str,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let module_key = self.resolve_module_qualifier(qualifier, caller_frame, span)?;
        let procedure = self
            .procedures
            .get(&qualified_key(Some(&module_key), name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Sub '{}.{}' is not defined", qualifier, name),
                    Some(span),
                )
            })?;
        if caller_frame.module_key() != Some(module_key.as_str())
            && !crate::modules::is_public(procedure.visibility)
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                format!("Sub '{}.{}' is Private", qualifier, name),
                Some(span),
            ));
        }
        let mut frame = self
            .module_frames
            .get(&module_key)
            .cloned()
            .expect("module loaded");
        frame.set_module_key(module_key);
        self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;
        match self.exec_block(&procedure.body, &mut frame)? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(procedure.span),
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

    fn resolve_function_module(
        &self,
        name: &str,
        frame: &Frame,
        span: Span,
    ) -> Result<Option<String>, Diagnostic> {
        if self.functions.contains_key(&key(name)) {
            return Ok(None);
        }
        if let Some(current) = frame.module_key()
            && self
                .functions
                .contains_key(&qualified_key(Some(current), name))
        {
            return Ok(Some(current.to_string()));
        }
        self.resolve_unqualified(name, frame, span, &self.function_modules, "Function")
    }

    fn resolve_sub_module(
        &self,
        name: &str,
        frame: &Frame,
        span: Span,
    ) -> Result<Option<String>, Diagnostic> {
        if self.procedures.contains_key(&key(name)) {
            return Ok(None);
        }
        if let Some(current) = frame.module_key()
            && self
                .procedures
                .contains_key(&qualified_key(Some(current), name))
        {
            return Ok(Some(current.to_string()));
        }
        self.resolve_unqualified(name, frame, span, &self.sub_modules, "Sub")
    }

    fn resolve_unqualified(
        &self,
        name: &str,
        frame: &Frame,
        span: Span,
        modules_by_name: &std::collections::HashMap<String, Vec<String>>,
        kind: &str,
    ) -> Result<Option<String>, Diagnostic> {
        let Some(modules) = modules_by_name.get(&key(name)) else {
            return Ok(None);
        };
        if let Some(current) = frame.module_key()
            && modules.iter().any(|module| module == current)
        {
            return Ok(Some(current.to_string()));
        }
        let imported: Vec<_> = frame
            .module_key()
            .and_then(|current| self.module_imports.get(current))
            .into_iter()
            .flatten()
            .filter(|import| modules.iter().any(|module| module == &import.module))
            .map(|import| import.module.clone())
            .collect();
        let candidates = if imported.is_empty() {
            modules.clone()
        } else {
            imported
        };
        if candidates.len() == 1 {
            Ok(Some(candidates[0].clone()))
        } else {
            Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::AMBIGUOUS_IMPORT,
                format!(
                    "{} '{}' is imported from multiple modules; use a module qualifier",
                    kind, name
                ),
                Some(span),
            ))
        }
    }

    pub(crate) fn resolve_module_qualifier(
        &self,
        qualifier: &str,
        frame: &Frame,
        span: Span,
    ) -> Result<String, Diagnostic> {
        let qualifier_key = key(qualifier);
        if let Some(current) = frame.module_key() {
            if current == qualifier_key {
                return Ok(current.to_string());
            }
            if let Some(imports) = self.module_imports.get(current)
                && let Some(import) = imports
                    .iter()
                    .find(|import| import.qualifier.eq_ignore_ascii_case(qualifier))
            {
                return Ok(import.module.clone());
            }
        }
        if self.module_frames.contains_key(&qualifier_key) {
            return Ok(qualifier_key);
        }
        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Module '{}' is not imported", qualifier),
            Some(span),
        ))
    }

    pub(crate) fn call_method_sub(
        &mut self,
        object: Value,
        method: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
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
        let procedure = class.subs.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&class.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, procedure.name));
        let result = self.exec_block(&procedure.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(procedure.span),
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

    pub(crate) fn call_method_sub_values(
        &mut self,
        object: Value,
        method: &str,
        args: &[Value],
        caller_frame: &mut Frame,
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
        let procedure = class.subs.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&class.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameter_values(&procedure.params, args, &mut frame, span)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, procedure.name));
        let result = self.exec_block(&procedure.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(procedure.span),
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

    pub(crate) fn call_method_function(
        &mut self,
        object: Value,
        method: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        // Handle VBA namespace: VBA.Join etc.
        // VBA evaluates to Empty in our current implementation of global objects.
        if matches!(object, Value::Empty)
            && let Some(val) =
                super::builtins::dispatch_function(self, method, args, caller_frame, span)?
        {
            return Ok(val);
        }

        let instance = ensure_object(object.clone(), span)?;
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
        if let Some(function) = class.functions.get(&key(method)).cloned() {
            let mut frame = Frame::default();
            frame.inherit_modules_from(caller_frame)?;
            if let Some((module_key, _)) = key(&class.name).split_once('.') {
                frame.set_module_key(module_key.to_string());
            }
            frame.declare_object_alias("me", &class.name, instance, span)?;
            self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;
            let return_type = self.resolve_type_name(&function.return_type, &frame, span)?;
            if !frame.has_variable(&function.name) {
                frame.declare(
                    &function.name,
                    return_type.clone(),
                    None,
                    self.option_base,
                    function.span,
                    &self.types,
                    &self.enums,
                )?;
            }
            self.scope_stack
                .push(format!("{}.{}", class.name, function.name));
            if function.is_iterator {
                frame.set_yield_mode();
            }
            let result = self.exec_block(&function.body, &mut frame);
            self.scope_stack.pop();
            return match result? {
                ControlFlow::Return(value) => {
                    if function.is_iterator {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::CONTROL_FLOW,
                            "Return is not allowed inside Iterator; use Yield or Exit Function",
                            Some(function.span),
                        ));
                    }
                    coerce_assignment(&return_type, value, span)
                }
                ControlFlow::Continue | ControlFlow::ExitFunction => {
                    if function.is_iterator {
                        let elements = frame.take_yielded_values().unwrap_or_default();
                        let len = elements.len() as i64;
                        Ok(Value::Array {
                            element_type: function.return_type.clone(),
                            elements,
                            bounds: vec![crate::runtime::ArrayBound {
                                lower: self.option_base,
                                upper: self.option_base + len - 1,
                            }],
                            allocated: true,
                        })
                    } else {
                        frame.get(&function.name, function.span)
                    }
                }
                ControlFlow::ExitSub => Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::CONTROL_FLOW,
                    "Exit Sub is only valid inside Sub",
                    Some(function.span),
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
        }

        if class.properties.contains_key(&key(method)) {
            // Try Case 1: The property itself takes these arguments
            if let Ok(value) =
                self.call_property_get(object.clone(), method, args, caller_frame, span)
            {
                return Ok(value);
            }

            // Try Case 2: The property returns an object that has a default property
            let value = self.call_property_get(object, method, &[], caller_frame, span)?;
            if let Value::Object(ref inner_object) = value {
                let inner_class_name = inner_object.borrow().class_name.clone();
                if let Some(default_prop_name) = self
                    .classes
                    .get(&key(&inner_class_name))
                    .and_then(|c| c.default_member.clone())
                {
                    return self.call_method_function(
                        value,
                        &default_prop_name,
                        args,
                        caller_frame,
                        span,
                    );
                }
            }
        }

        Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
            format!(
                "Class '{}' has no method or property '{}'",
                class.name, method
            ),
            Some(span),
        ))
    }

    pub(crate) fn call_method_function_decl(
        &mut self,
        object: Value,
        function: Function,
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
        if !function.params.is_empty() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                format!("Iterator '{}' must not have parameters", function.name),
                Some(function.span),
            ));
        }
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        if let Some((module_key, _)) = key(&class.name).split_once('.') {
            frame.set_module_key(module_key.to_string());
        }
        frame.declare_object_alias("me", &class.name, instance, span)?;
        let return_type = self.resolve_type_name(&function.return_type, &frame, span)?;
        if !frame.has_variable(&function.name) {
            frame.declare(
                &function.name,
                return_type.clone(),
                None,
                self.option_base,
                function.span,
                &self.types,
                &self.enums,
            )?;
        }
        self.scope_stack
            .push(format!("{}.{}", class.name, function.name));
        if function.is_iterator {
            frame.set_yield_mode();
        }
        let result = self.exec_block(&function.body, &mut frame);
        self.scope_stack.pop();
        let result = match result? {
            ControlFlow::Return(value) => {
                if function.is_iterator {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::CONTROL_FLOW,
                        "Return is not allowed inside Iterator; use Yield or Exit Function",
                        Some(function.span),
                    ));
                }
                coerce_assignment(&return_type, value, span)
            }
            ControlFlow::Continue | ControlFlow::ExitFunction => {
                if function.is_iterator {
                    let elements = frame.take_yielded_values().unwrap_or_default();
                    let len = elements.len() as i64;
                    Ok(Value::Array {
                        element_type: function.return_type.clone(),
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: self.option_base,
                            upper: self.option_base + len - 1,
                        }],
                        allocated: true,
                    })
                } else {
                    frame.get(&function.name, function.span)
                }
            }
            ControlFlow::ExitSub => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Sub is only valid inside Sub",
                Some(function.span),
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

    pub(crate) fn bind_parameters(
        &mut self,
        params: &[crate::Parameter],
        args: &[Expr],
        caller_frame: &mut Frame,
        callee_frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        let required = params
            .iter()
            .filter(|param| !param.is_optional && !param.is_param_array)
            .count();
        let has_param_array = params.last().is_some_and(|param| param.is_param_array);
        if args.len() < required || (!has_param_array && args.len() > params.len()) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                format!("Expected {} argument(s), got {}", required, args.len()),
                args.first().map(|arg| arg.span),
            ));
        }
        let mut ordered: Vec<Option<&Expr>> = vec![None; params.len()];
        let mut paramarray_args = Vec::new();
        let mut positional_index = 0;
        let mut saw_named = false;
        for arg in args {
            if let ExprKind::NamedArg { name, expr } = &arg.kind {
                saw_named = true;
                let Some(index) = params
                    .iter()
                    .position(|param| param.name.eq_ignore_ascii_case(name))
                else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Unknown named argument '{}'", name),
                        Some(arg.span),
                    ));
                };
                if params[index].is_param_array {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::ARRAY,
                        "ParamArray arguments cannot be supplied by name",
                        Some(arg.span),
                    ));
                }
                if ordered[index].is_some() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        format!("Argument '{}' is specified more than once", name),
                        Some(arg.span),
                    ));
                }
                ordered[index] = Some(expr);
                continue;
            }
            if saw_named {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::GENERIC,
                    "Positional arguments cannot appear after named arguments",
                    Some(arg.span),
                ));
            }
            if positional_index < params.len() && params[positional_index].is_param_array {
                paramarray_args.push(arg);
            } else if positional_index < params.len() {
                ordered[positional_index] = Some(arg);
                positional_index += 1;
            } else if has_param_array {
                paramarray_args.push(arg);
            } else {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::PARSE,
                    format!("Expected {} argument(s), got {}", params.len(), args.len()),
                    Some(arg.span),
                ));
            }
        }

        for (index, param) in params.iter().enumerate() {
            let param_ty = self.resolve_type_name(&param.ty, callee_frame, param.span)?;
            if param.is_param_array {
                let mut elements = Vec::new();
                for arg in &paramarray_args {
                    elements.push(self.eval_expr(arg, caller_frame)?);
                }
                callee_frame.declare(
                    &param.name,
                    param_ty.clone(),
                    Some(crate::ArrayDecl::Dynamic),
                    self.option_base,
                    param.span,
                    &self.types,
                    &self.enums,
                )?;
                let len = elements.len();
                let _ = callee_frame.assign(
                    &param.name,
                    Value::Array {
                        element_type: param.ty.clone(),
                        elements,
                        bounds: vec![crate::runtime::ArrayBound {
                            lower: self.option_base,
                            upper: self.option_base + len as i64 - 1,
                        }],
                        allocated: true,
                    },
                    param.span,
                )?;

                continue;
            }
            let arg = ordered[index];
            match param.mode {
                PassingMode::ByVal => {
                    let value = if let Some(arg) = arg {
                        self.eval_expr(arg, caller_frame)?
                    } else if let Some(default) = &param.optional_default {
                        self.eval_expr(default, caller_frame)?
                    } else {
                        Value::Missing
                    };
                    callee_frame.declare(
                        &param.name,
                        param_ty,
                        None,
                        self.option_base,
                        param.span,
                        &self.types,
                        &self.enums,
                    )?;
                    if matches!(value, Value::Missing) {
                        callee_frame.assign_missing(&param.name, param.span)?;
                    } else {
                        let _ = callee_frame.assign(&param.name, value, param.span)?;
                    }
                }
                PassingMode::ByRef => {
                    let Some(arg) = arg else {
                        let value = if let Some(default) = &param.optional_default {
                            self.eval_expr(default, caller_frame)?
                        } else {
                            Value::Missing
                        };
                        callee_frame.declare(
                            &param.name,
                            param_ty.clone(),
                            None,
                            self.option_base,
                            param.span,
                            &self.types,
                            &self.enums,
                        )?;
                        if matches!(value, Value::Missing) {
                            callee_frame.assign_missing(&param.name, param.span)?;
                        } else {
                            let _ = callee_frame.assign(&param.name, value, param.span)?;
                        }
                        continue;
                    };
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    callee_frame.declare_alias(&param.name, param_ty, variable, param.span)?;
                }
            }
        }
        Ok(())
    }

    fn bind_parameter_values(
        &mut self,
        params: &[crate::Parameter],
        args: &[Value],
        callee_frame: &mut Frame,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if args.len() != params.len() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                format!("Expected {} argument(s), got {}", params.len(), args.len()),
                Some(span),
            ));
        }
        for (param, value) in params.iter().zip(args.iter()) {
            let param_ty = self.resolve_type_name(&param.ty, callee_frame, param.span)?;
            callee_frame.declare(
                &param.name,
                param_ty,
                None,
                self.option_base,
                param.span,
                &self.types,
                &self.enums,
            )?;
            let _ = callee_frame.assign(&param.name, value.clone(), param.span)?;
        }
        Ok(())
    }
}

fn qualified_key(module_key: Option<&str>, name: &str) -> String {
    match module_key {
        Some(module_key) => format!("{}::{}", module_key, key(name)),
        None => key(name),
    }
}
