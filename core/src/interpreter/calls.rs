use crate::runtime::{Diagnostic, Span, Value};
use crate::{Expr, ExprKind, PassingMode};

use super::objects::ensure_object;
use super::values::{coerce_assignment, default_value, key};
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn call_function(
        &mut self,
        name: &str,
        args: &[Expr],
        caller_frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let function = self.functions.get(&key(name)).cloned().ok_or_else(|| {
            Diagnostic::new(format!("Function '{}' is not defined", name), Some(span))
        })?;

        self.call_stack
            .push(format!("Function '{}'", function.name));
        self.scope_stack.push(format!("Function {}", function.name));
        let result = (|| {
            let mut frame = Frame::default();
            frame.inherit_modules_from(caller_frame)?;
            self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;

            match self.exec_block(&function.body, &mut frame)? {
                ControlFlow::Return(value) => coerce_assignment(&function.return_type, value, span),
                ControlFlow::Continue => Err(Diagnostic::new(
                    format!("Function '{}' must return a value", function.name),
                    Some(function.span),
                )),
                ControlFlow::ExitFunction => {
                    default_value(&function.return_type, &self.types, &self.enums, span)
                }
                ControlFlow::ExitSub => Err(Diagnostic::new(
                    "Exit Sub is only valid inside Sub",
                    Some(function.span),
                )),
                ControlFlow::ExitFor
                | ControlFlow::ExitWhile
                | ControlFlow::ExitDo
                | ControlFlow::GoTo(_)
                | ControlFlow::Resume(_) => Err(Diagnostic::new(
                    "Exit statement escaped its block",
                    Some(span),
                )),
            }
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
        let procedure =
            self.procedures.get(&key(name)).cloned().ok_or_else(|| {
                Diagnostic::new(format!("Sub '{}' is not defined", name), Some(span))
            })?;

        self.call_stack.push(format!("Sub '{}'", procedure.name));
        self.scope_stack.push(format!("Sub {}", procedure.name));
        let result = (|| {
            let mut frame = Frame::default();
            frame.inherit_modules_from(caller_frame)?;
            self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;

            match self.exec_block(&procedure.body, &mut frame)? {
                ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
                ControlFlow::Return(_) => Err(Diagnostic::new(
                    "Return is only allowed inside Function",
                    Some(procedure.span),
                )),
                ControlFlow::ExitFunction => Err(Diagnostic::new(
                    "Exit Function is only valid inside Function",
                    Some(procedure.span),
                )),
                ControlFlow::ExitFor
                | ControlFlow::ExitWhile
                | ControlFlow::ExitDo
                | ControlFlow::GoTo(_)
                | ControlFlow::Resume(_) => Err(Diagnostic::new(
                    "Exit statement escaped its block",
                    Some(span),
                )),
            }
        })();
        let result = result.map_err(|diagnostic| self.with_stack_context(diagnostic));
        self.scope_stack.pop();
        self.call_stack.pop();
        result
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
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let procedure = class.subs.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameters(&procedure.params, args, caller_frame, &mut frame)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, procedure.name));
        let result = self.exec_block(&procedure.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                "Exit Function is only valid inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
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
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let procedure = class.subs.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameter_values(&procedure.params, args, &mut frame, span)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, procedure.name));
        let result = self.exec_block(&procedure.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(()),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                "Exit Function is only valid inside Function",
                Some(procedure.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
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
        let instance = ensure_object(object, span)?;
        let class_name = instance.borrow().class_name.clone();
        let class = self
            .classes
            .get(&key(&class_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(format!("Class '{}' is not defined", class_name), Some(span))
            })?;
        let function = class.functions.get(&key(method)).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("Class '{}' has no method '{}'", class.name, method),
                Some(span),
            )
        })?;
        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        frame.declare_object_alias("me", &class.name, instance, span)?;
        self.bind_parameters(&function.params, args, caller_frame, &mut frame)?;
        self.scope_stack
            .push(format!("{}.{}", class.name, function.name));
        let result = self.exec_block(&function.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Return(value) => coerce_assignment(&function.return_type, value, span),
            ControlFlow::Continue => Err(Diagnostic::new(
                format!("Function '{}' must return a value", function.name),
                Some(function.span),
            )),
            ControlFlow::ExitFunction => {
                default_value(&function.return_type, &self.types, &self.enums, span)
            }
            ControlFlow::ExitSub => Err(Diagnostic::new(
                "Exit Sub is only valid inside Sub",
                Some(function.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
                "Exit statement escaped its block",
                Some(span),
            )),
        }
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
                        format!("Unknown named argument '{}'", name),
                        Some(arg.span),
                    ));
                };
                if params[index].is_param_array {
                    return Err(Diagnostic::new(
                        "ParamArray arguments cannot be supplied by name",
                        Some(arg.span),
                    ));
                }
                if ordered[index].is_some() {
                    return Err(Diagnostic::new(
                        format!("Argument '{}' is specified more than once", name),
                        Some(arg.span),
                    ));
                }
                ordered[index] = Some(expr);
                continue;
            }
            if saw_named {
                return Err(Diagnostic::new(
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
                    format!("Expected {} argument(s), got {}", params.len(), args.len()),
                    Some(arg.span),
                ));
            }
        }

        for (index, param) in params.iter().enumerate() {
            if param.is_param_array {
                let mut elements = Vec::new();
                for arg in &paramarray_args {
                    elements.push(self.eval_expr(arg, caller_frame)?);
                }
                callee_frame.declare(
                    &param.name,
                    param.ty.clone(),
                    Some(crate::ArrayDecl::Dynamic),
                    self.option_base,
                    param.span,
                    &self.types,
                    &self.enums,
                )?;
                callee_frame.assign(
                    &param.name,
                    Value::Array {
                        element_type: param.ty.clone(),
                        elements,
                        lower_bound: self.option_base,
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
                        param.ty.clone(),
                        None,
                        self.option_base,
                        param.span,
                        &self.types,
                        &self.enums,
                    )?;
                    if matches!(value, Value::Missing) {
                        callee_frame.assign_missing(&param.name, param.span)?;
                    } else {
                        callee_frame.assign(&param.name, value, param.span)?;
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
                            param.ty.clone(),
                            None,
                            self.option_base,
                            param.span,
                            &self.types,
                            &self.enums,
                        )?;
                        if matches!(value, Value::Missing) {
                            callee_frame.assign_missing(&param.name, param.span)?;
                        } else {
                            callee_frame.assign(&param.name, value, param.span)?;
                        }
                        continue;
                    };
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    callee_frame.declare_alias(
                        &param.name,
                        param.ty.clone(),
                        variable,
                        param.span,
                    )?;
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
                format!("Expected {} argument(s), got {}", params.len(), args.len()),
                Some(span),
            ));
        }
        for (param, value) in params.iter().zip(args.iter()) {
            callee_frame.declare(
                &param.name,
                param.ty.clone(),
                None,
                self.option_base,
                param.span,
                &self.types,
                &self.enums,
            )?;
            callee_frame.assign(&param.name, value.clone(), param.span)?;
        }
        Ok(())
    }
}
