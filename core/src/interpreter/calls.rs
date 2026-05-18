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
            .filter(|param| param.optional_default.is_none() && !param.is_param_array)
            .count();
        let has_param_array = params.last().is_some_and(|param| param.is_param_array);
        if args.len() < required || (!has_param_array && args.len() > params.len()) {
            return Err(Diagnostic::new(
                format!("Expected {} argument(s), got {}", required, args.len()),
                args.first().map(|arg| arg.span),
            ));
        }
        let mut arg_index = 0;
        for param in params {
            if param.is_param_array {
                let mut elements = Vec::new();
                while let Some(arg) = args.get(arg_index) {
                    elements.push(self.eval_expr(arg, caller_frame)?);
                    arg_index += 1;
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
            let arg = args.get(arg_index);
            arg_index += usize::from(arg.is_some());
            match param.mode {
                PassingMode::ByVal => {
                    let value = if let Some(arg) = arg {
                        self.eval_expr(arg, caller_frame)?
                    } else {
                        self.eval_expr(
                            param.optional_default.as_ref().expect("validated"),
                            caller_frame,
                        )?
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
                    callee_frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
                    let Some(arg) = arg else {
                        let value = self.eval_expr(
                            param.optional_default.as_ref().expect("validated"),
                            caller_frame,
                        )?;
                        callee_frame.declare(
                            &param.name,
                            param.ty.clone(),
                            None,
                            self.option_base,
                            param.span,
                            &self.types,
                            &self.enums,
                        )?;
                        callee_frame.assign(&param.name, value, param.span)?;
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
}
