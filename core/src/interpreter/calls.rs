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

        if args.len() != function.params.len() {
            return Err(Diagnostic::new(
                format!(
                    "Function '{}' expects {} argument(s), got {}",
                    function.name,
                    function.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        }

        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        for (param, arg) in function.params.iter().zip(args) {
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, caller_frame)?;
                    frame.declare(
                        &param.name,
                        param.ty.clone(),
                        None,
                        param.span,
                        &self.types,
                        &self.enums,
                    )?;
                    frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    frame.declare_alias(&param.name, param.ty.clone(), variable, param.span)?;
                }
            }
        }

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
            ControlFlow::ExitFor | ControlFlow::ExitWhile | ControlFlow::ExitDo => Err(
                Diagnostic::new("Exit statement escaped its block", Some(span)),
            ),
        }
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

        if args.len() != procedure.params.len() {
            return Err(Diagnostic::new(
                format!(
                    "Sub '{}' expects {} argument(s), got {}",
                    procedure.name,
                    procedure.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        }

        let mut frame = Frame::default();
        frame.inherit_modules_from(caller_frame)?;
        for (param, arg) in procedure.params.iter().zip(args) {
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, caller_frame)?;
                    frame.declare(
                        &param.name,
                        param.ty.clone(),
                        None,
                        param.span,
                        &self.types,
                        &self.enums,
                    )?;
                    frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
                    let ExprKind::Variable(arg_name) = &arg.kind else {
                        return Err(Diagnostic::new(
                            "ByRef argument must be a variable",
                            Some(arg.span),
                        ));
                    };
                    let variable = caller_frame.variable(arg_name, arg.span)?;
                    frame.declare_alias(&param.name, param.ty.clone(), variable, param.span)?;
                }
            }
        }

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
            ControlFlow::ExitFor | ControlFlow::ExitWhile | ControlFlow::ExitDo => Err(
                Diagnostic::new("Exit statement escaped its block", Some(span)),
            ),
        }
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
            ControlFlow::ExitFor | ControlFlow::ExitWhile | ControlFlow::ExitDo => Err(
                Diagnostic::new("Exit statement escaped its block", Some(span)),
            ),
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
            ControlFlow::ExitFor | ControlFlow::ExitWhile | ControlFlow::ExitDo => Err(
                Diagnostic::new("Exit statement escaped its block", Some(span)),
            ),
        }
    }

    pub(crate) fn bind_parameters(
        &mut self,
        params: &[crate::Parameter],
        args: &[Expr],
        caller_frame: &mut Frame,
        callee_frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        if args.len() != params.len() {
            return Err(Diagnostic::new(
                format!("Expected {} argument(s), got {}", params.len(), args.len()),
                args.first().map(|arg| arg.span),
            ));
        }
        for (param, arg) in params.iter().zip(args) {
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, caller_frame)?;
                    callee_frame.declare(
                        &param.name,
                        param.ty.clone(),
                        None,
                        param.span,
                        &self.types,
                        &self.enums,
                    )?;
                    callee_frame.assign(&param.name, value, param.span)?;
                }
                PassingMode::ByRef => {
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
