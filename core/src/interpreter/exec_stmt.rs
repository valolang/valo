use crate::runtime::{Diagnostic, RuntimeErrorInfo, Value};
use crate::{
    AssignTarget, CaseCompareOp, CaseItem, DoLoopCondition, ExitTarget, OnErrorMode, ResumeTarget,
    Stmt,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::values::{compare_case_values, key, values_equal};
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn exec_block(
        &mut self,
        statements: &[Stmt],
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        let labels = index_labels(statements);
        let line_numbers = index_line_numbers(statements);
        let mut ip = 0;
        while ip < statements.len() {
            match self.exec_stmt(&statements[ip], frame) {
                Ok(ControlFlow::Continue) => ip += 1,
                Ok(ControlFlow::GoTo(label)) => {
                    ip = labels
                        .get(&super::values::key(&label))
                        .copied()
                        .expect("GoTo label validated");
                }
                Ok(ControlFlow::Resume(target)) => {
                    let Some(failing_ip) = frame.handled_error_ip() else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            "Resume is only valid after a handled runtime error",
                            Some(stmt_span(&statements[ip])),
                        )
                        .with_primary_label("no handled error is active"));
                    };
                    ip = match target {
                        ResumeTarget::Retry => failing_ip,
                        ResumeTarget::Next => failing_ip + 1,
                        ResumeTarget::Label(label) => labels
                            .get(&super::values::key(&label))
                            .copied()
                            .expect("Resume label validated"),
                    };
                    frame.clear_handled_error();
                }
                Ok(flow) => return Ok(flow),
                Err(error) if frame.resume_next() => {
                    self.set_err(&error, line_numbers.get(&ip).copied().unwrap_or(0));
                    ip += 1;
                }
                Err(error)
                    if frame.error_handler().is_some() && frame.handled_error_ip().is_none() =>
                {
                    self.set_err(&error, line_numbers.get(&ip).copied().unwrap_or(0));
                    frame.set_handled_error_ip(ip);
                    let handler = frame.error_handler().expect("checked").to_string();
                    ip = labels
                        .get(&super::values::key(&handler))
                        .copied()
                        .expect("On Error label validated");
                }
                Err(error) => return Err(error),
            }
        }
        Ok(ControlFlow::Continue)
    }

    pub(crate) fn exec_stmt(
        &mut self,
        stmt: &Stmt,
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        match stmt {
            Stmt::Dim {
                name,
                ty,
                array,
                as_new,
                span,
            } => {
                let ty = self.resolve_type_name(ty, frame, *span)?;
                let ty_for_new = ty.clone();
                frame.declare(
                    name,
                    ty,
                    array.clone(),
                    self.option_base,
                    *span,
                    &self.types,
                    &self.enums,
                )?;
                if *as_new {
                    let crate::runtime::TypeName::User(class_name) = ty_for_new else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                            "As New requires a class type",
                            Some(*span),
                        ));
                    };
                    let value = self.new_object(&class_name, &[], frame, *span)?;
                    let _ = frame.assign(name, value, *span)?;
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::Static {
                name,
                ty,
                array,
                as_new,
                span,
            } => {
                if *as_new {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "Static As New is not supported",
                        Some(*span),
                    ));
                }
                let ty = self.resolve_type_name(ty, frame, *span)?;
                let scope = self
                    .scope_stack
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "<module>".to_string());
                let static_frame = self.static_frames.entry(scope).or_default();
                frame.declare_static(
                    name,
                    ty,
                    array.clone(),
                    self.option_base,
                    *span,
                    &self.types,
                    &self.enums,
                    static_frame,
                )?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Const {
                name,
                ty,
                value,
                span,
            } => {
                let value = self.eval_expr(value, frame)?;
                let ty = ty.clone().unwrap_or_else(|| value.type_name());
                let ty = self.resolve_type_name(&ty, frame, *span)?;
                frame.declare_const(name, ty, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Assign { target, expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                self.assign_target(target, value, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::SetAssign { target, expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                self.assign_target(target, value, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::ConsoleWriteLine { args, .. } => {
                let mut parts = Vec::new();
                for arg in args {
                    let value = self.eval_expr(arg, frame)?;
                    if matches!(value, Value::Missing) {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::GENERIC,
                            "Missing optional argument cannot be used as a value",
                            Some(arg.span),
                        ));
                    }
                    parts.push(
                        self.resolve_default_value(value, frame, arg.span)?
                            .to_output_string(),
                    );
                }
                self.output.push(parts.join(" "));
                Ok(ControlFlow::Continue)
            }
            Stmt::SubCall { name, args, span } => {
                self.call_sub(name, args, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::MemberSubCall {
                object,
                method,
                args,
                span,
            } => {
                if let crate::ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Console")
                    && method.eq_ignore_ascii_case("WriteLine")
                {
                    let mut parts = Vec::new();
                    for arg in args {
                        let value = self.eval_expr(arg, frame)?;
                        parts.push(
                            self.resolve_default_value(value, frame, arg.span)?
                                .to_output_string(),
                        );
                    }
                    self.output.push(parts.join(" "));
                    return Ok(ControlFlow::Continue);
                }
                if let crate::ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Err")
                    && method.eq_ignore_ascii_case("Clear")
                    && args.is_empty()
                {
                    self.clear_err();
                    return Ok(ControlFlow::Continue);
                }
                if let crate::ExprKind::Variable(name) = &object.kind
                    && name.eq_ignore_ascii_case("Err")
                    && method.eq_ignore_ascii_case("Raise")
                {
                    return Err(self.err_raise(args, frame, *span)?);
                }
                if let crate::ExprKind::Variable(module_name) = &object.kind
                    && self
                        .resolve_module_qualifier(module_name, frame, *span)
                        .is_ok()
                {
                    self.call_module_sub(module_name, method, args, frame, *span)?;
                    return Ok(ControlFlow::Continue);
                }
                let object = self.eval_expr(object, frame)?;
                self.call_method_sub(object, method, args, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::RaiseEvent { name, args, span } => {
                self.raise_event(name, args, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Return { expr, .. } => {
                let value = self.eval_expr(expr, frame)?;
                Ok(ControlFlow::Return(value))
            }
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                ..
            } => {
                if self.eval_expr(condition, frame)?.is_truthy() {
                    self.exec_block(then_body, frame)
                } else {
                    for branch in elseif_branches {
                        if self.eval_expr(&branch.condition, frame)?.is_truthy() {
                            return self.exec_block(&branch.body, frame);
                        }
                    }
                    self.exec_block(else_body, frame)
                }
            }
            Stmt::SelectCase {
                subject,
                branches,
                else_body,
                ..
            } => {
                let subject = self.eval_expr(subject, frame)?;
                for branch in branches {
                    for item in &branch.items {
                        if self.case_item_matches(&subject, item, frame)? {
                            return self.exec_block(&branch.body, frame);
                        }
                    }
                }
                self.exec_block(else_body, frame)
            }
            Stmt::While {
                condition, body, ..
            } => {
                while self.eval_expr(condition, frame)?.is_truthy() {
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        ControlFlow::ExitWhile => return Ok(ControlFlow::Continue),
                        flow @ ControlFlow::Return(_) => return Ok(flow),
                        flow => return Ok(flow),
                    }
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::DoLoop {
                condition, body, ..
            } => {
                match condition {
                    DoLoopCondition::PreWhile(condition) => {
                        while self.eval_expr(condition, frame)?.is_truthy() {
                            match self.exec_block(body, frame)? {
                                ControlFlow::Continue => {}
                                ControlFlow::ExitDo => return Ok(ControlFlow::Continue),
                                flow @ ControlFlow::Return(_) => return Ok(flow),
                                flow => return Ok(flow),
                            }
                        }
                    }
                    DoLoopCondition::PreUntil(condition) => {
                        while !self.eval_expr(condition, frame)?.is_truthy() {
                            match self.exec_block(body, frame)? {
                                ControlFlow::Continue => {}
                                ControlFlow::ExitDo => return Ok(ControlFlow::Continue),
                                flow @ ControlFlow::Return(_) => return Ok(flow),
                                flow => return Ok(flow),
                            }
                        }
                    }
                    DoLoopCondition::PostWhile(condition) => loop {
                        match self.exec_block(body, frame)? {
                            ControlFlow::Continue => {}
                            ControlFlow::ExitDo => return Ok(ControlFlow::Continue),
                            flow @ ControlFlow::Return(_) => return Ok(flow),
                            flow => return Ok(flow),
                        }
                        if !self.eval_expr(condition, frame)?.is_truthy() {
                            break;
                        }
                    },
                    DoLoopCondition::PostUntil(condition) => loop {
                        match self.exec_block(body, frame)? {
                            ControlFlow::Continue => {}
                            ControlFlow::ExitDo => return Ok(ControlFlow::Continue),
                            flow @ ControlFlow::Return(_) => return Ok(flow),
                            flow => return Ok(flow),
                        }
                        if self.eval_expr(condition, frame)?.is_truthy() {
                            break;
                        }
                    },
                    DoLoopCondition::Infinite => loop {
                        match self.exec_block(body, frame)? {
                            ControlFlow::Continue => {}
                            ControlFlow::ExitDo => return Ok(ControlFlow::Continue),
                            flow @ ControlFlow::Return(_) => return Ok(flow),
                            flow => return Ok(flow),
                        }
                    },
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::For {
                variable,
                start,
                end,
                step,
                body,
                span,
                ..
            } => {
                let mut current =
                    self.eval_integer_expr(start, frame, "For start value must be Integer")?;
                let end = self.eval_integer_expr(end, frame, "For end value must be Integer")?;
                let step = match step {
                    Some(step) => {
                        self.eval_integer_expr(step, frame, "For step value must be Integer")?
                    }
                    None => 1,
                };

                if step == 0 {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::GENERIC,
                        "For Step cannot be zero",
                        Some(*span),
                    ));
                }

                loop {
                    if (step > 0 && current > end) || (step < 0 && current < end) {
                        break;
                    }

                    let old = frame.assign(variable, Value::Integer(current), *span)?;
                    self.maybe_terminate(old, *span)?;
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        ControlFlow::ExitFor => return Ok(ControlFlow::Continue),
                        flow @ ControlFlow::Return(_) => return Ok(flow),
                        flow => return Ok(flow),
                    }
                    current += step;
                }

                Ok(ControlFlow::Continue)
            }
            Stmt::ForEach {
                variable,
                iterable,
                body,
                span,
                ..
            } => {
                let iterable = self.eval_expr(iterable, frame)?;
                let values = super::arrays::array_values(&iterable, *span)?;
                for value in values {
                    let old = frame.assign(variable, value, *span)?;
                    self.maybe_terminate(old, *span)?;
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        ControlFlow::ExitFor => return Ok(ControlFlow::Continue),
                        flow @ ControlFlow::Return(_) => return Ok(flow),
                        flow => return Ok(flow),
                    }
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::ReDim {
                name,
                lower_bound,
                upper_bound,
                preserve,
                span,
            } => {
                let lower_bound = if let Some(lower_bound) = lower_bound {
                    self.eval_integer_expr(lower_bound, frame, "ReDim lower bound must be Integer")?
                } else {
                    self.option_base
                };
                let upper_bound = self.eval_integer_expr(
                    upper_bound,
                    frame,
                    "ReDim upper bound must be Integer",
                )?;
                frame.redim_array(
                    name,
                    upper_bound,
                    lower_bound,
                    *preserve,
                    &self.types,
                    &self.enums,
                    *span,
                )?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Erase { name, span } => {
                frame.erase_array(name, *span, &self.types, &self.enums)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Label { .. } => Ok(ControlFlow::Continue),
            Stmt::GoTo { label, .. } => Ok(ControlFlow::GoTo(label.clone())),
            Stmt::OnError { mode, .. } => {
                match mode {
                    OnErrorMode::ResumeNext => frame.set_resume_next(true),
                    OnErrorMode::GoToZero => {
                        frame.set_resume_next(false);
                        frame.set_error_handler(None);
                    }
                    OnErrorMode::GoToMinusOne => {
                        frame.clear_handled_error();
                        self.erl = 0;
                    }
                    OnErrorMode::GoToLabel(label) => {
                        frame.set_error_handler(Some(label.clone()));
                    }
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::Resume { target, .. } => Ok(ControlFlow::Resume(target.clone())),
            Stmt::With { target, body, .. } => {
                let target = self.eval_expr(target, frame)?;
                frame.push_with_target(target);
                let flow = self.exec_block(body, frame);
                frame.pop_with_target();
                flow
            }
            Stmt::Exit { target, .. } => match target {
                ExitTarget::Sub => Ok(ControlFlow::ExitSub),
                ExitTarget::Function => Ok(ControlFlow::ExitFunction),
                ExitTarget::For => Ok(ControlFlow::ExitFor),
                ExitTarget::While => Ok(ControlFlow::ExitWhile),
                ExitTarget::Do => Ok(ControlFlow::ExitDo),
            },
            Stmt::TryCatch {
                try_body,
                catch_block,
                finally_body,
                ..
            } => {
                let mut result = self.exec_block(try_body, frame);

                if let Err(error) = &result
                    && let Some(catch) = catch_block
                {
                    if let Some(var_name) = &catch.variable {
                        let error_obj = self.create_error_object(error);
                        frame.declare(
                            var_name,
                            crate::runtime::TypeName::User("Error".to_string()),
                            None,
                            self.option_base,
                            catch.span,
                            &self.types,
                            &self.enums,
                        )?;
                        frame.assign(var_name, error_obj, catch.span)?;
                    }
                    result = self.exec_block(&catch.body, frame);
                }

                if let Some(finally_body) = finally_body {
                    let finally_result = self.exec_block(finally_body, frame);
                    if let Err(error) = finally_result {
                        return Err(error);
                    }
                    if let Ok(flow) = finally_result
                        && !matches!(flow, ControlFlow::Continue)
                    {
                        return Ok(flow);
                    }
                }

                result
            }
            Stmt::DebugPrint { args, .. } => {
                let mut parts = Vec::new();
                for arg in args {
                    let value = self.eval_expr(arg, frame)?;
                    parts.push(
                        self.resolve_default_value(value, frame, arg.span)?
                            .to_output_string(),
                    );
                }
                self.output.push(parts.join("\t"));
                Ok(ControlFlow::Continue)
            }
        }
    }

    fn create_error_object(&self, diagnostic: &Diagnostic) -> Value {
        let mut fields = HashMap::new();
        if let Some(info) = &diagnostic.runtime_error {
            fields.insert(key("Number"), Value::Integer(info.number));
            fields.insert(key("Message"), Value::String(info.description.clone()));
            fields.insert(key("Description"), Value::String(info.description.clone()));
            fields.insert(key("Source"), Value::String(info.source.clone()));
            fields.insert(key("HelpFile"), Value::String(info.help_file.clone()));
            fields.insert(key("HelpContext"), Value::Integer(info.help_context));
        } else {
            fields.insert(key("Number"), Value::Integer(1));
            fields.insert(
                key("Message"),
                Value::String(diagnostic.message.to_string()),
            );
            fields.insert(
                key("Description"),
                Value::String(diagnostic.message.to_string()),
            );
            fields.insert(key("Source"), Value::String("Valo.Runtime".to_string()));
            fields.insert(key("HelpFile"), Value::String(String::new()));
            fields.insert(key("HelpContext"), Value::Integer(0));
        }

        Value::Object(Rc::new(RefCell::new(crate::runtime::ObjectValue {
            class_name: "Error".to_string(),
            fields,
            event_bindings: Vec::new(),
            terminated: false,
        })))
    }

    fn assign_target(
        &mut self,
        target: &AssignTarget,
        value: Value,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        match target {
            AssignTarget::Variable { name, .. } => {
                if frame.has_variable(name) {
                    let old = frame.assign(name, value, span)?;
                    self.maybe_terminate(old, span)
                } else {
                    let owner = frame.get("me", span)?;
                    self.assign_bare_class_field(owner, name, value, span)
                }
            }
            AssignTarget::ArrayElement { name, index, .. } => {
                let index = self.eval_integer_expr(index, frame, "Array index must be Integer")?;
                let old = frame.assign_array_element(name, index, value, span)?;
                self.maybe_terminate(old, span)
            }
            AssignTarget::Member { object, field, .. } => {
                self.assign_member(object, field, value, frame, span)
            }
        }
    }

    fn case_item_matches(
        &mut self,
        subject: &Value,
        item: &CaseItem,
        frame: &mut Frame,
    ) -> Result<bool, Diagnostic> {
        match item {
            CaseItem::Value(value) => {
                let value = self.eval_expr(value, frame)?;
                Ok(values_equal(subject, &value, self.option_compare))
            }
            CaseItem::Range { start, end } => {
                let start_value = self.eval_expr(start, frame)?;
                let end_value = self.eval_expr(end, frame)?;
                let lower = compare_case_values(
                    subject.clone(),
                    CaseCompareOp::GreaterEqual,
                    start_value,
                    self.option_compare,
                    start.span,
                )?;
                let upper = compare_case_values(
                    subject.clone(),
                    CaseCompareOp::LessEqual,
                    end_value,
                    self.option_compare,
                    end.span,
                )?;
                Ok(lower.is_truthy() && upper.is_truthy())
            }
            CaseItem::Compare { op, expr } => {
                let value = self.eval_expr(expr, frame)?;
                Ok(compare_case_values(
                    subject.clone(),
                    *op,
                    value,
                    self.option_compare,
                    expr.span,
                )?
                .is_truthy())
            }
        }
    }

    fn err_raise(
        &mut self,
        args: &[crate::Expr],
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<Diagnostic, Diagnostic> {
        let number = self.eval_integer_expr(&args[0], frame, "Err.Raise number must be Integer")?;
        let source = if let Some(arg) = args.get(1) {
            self.eval_string_arg(arg, frame, "Err.Raise source must be String")?
        } else {
            String::new()
        };
        let description = if let Some(arg) = args.get(2) {
            self.eval_string_arg(arg, frame, "Err.Raise description must be String")?
        } else {
            "Application-defined or object-defined error".to_string()
        };
        let help_file = if let Some(arg) = args.get(3) {
            self.eval_string_arg(arg, frame, "Err.Raise helpFile must be String")?
        } else {
            String::new()
        };
        let help_context = if let Some(arg) = args.get(4) {
            self.eval_integer_expr(arg, frame, "Err.Raise helpContext must be Integer")?
        } else {
            0
        };

        Ok(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            description.clone(),
            Some(span),
        )
        .with_runtime_error(RuntimeErrorInfo {
            number,
            source,
            description,
            help_file,
            help_context,
        }))
    }

    fn eval_string_arg(
        &mut self,
        expr: &crate::Expr,
        frame: &mut Frame,
        message: &str,
    ) -> Result<String, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            Value::String(value) => Ok(value),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                message,
                Some(expr.span),
            )),
        }
    }
}

fn stmt_span(stmt: &Stmt) -> crate::runtime::Span {
    match stmt {
        Stmt::Dim { span, .. }
        | Stmt::Static { span, .. }
        | Stmt::Const { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::SetAssign { span, .. }
        | Stmt::ConsoleWriteLine { span, .. }
        | Stmt::SubCall { span, .. }
        | Stmt::MemberSubCall { span, .. }
        | Stmt::RaiseEvent { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::If { span, .. }
        | Stmt::SelectCase { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoLoop { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForEach { span, .. }
        | Stmt::ReDim { span, .. }
        | Stmt::Erase { span, .. }
        | Stmt::Label { span, .. }
        | Stmt::GoTo { span, .. }
        | Stmt::OnError { span, .. }
        | Stmt::Resume { span, .. }
        | Stmt::With { span, .. }
        | Stmt::Exit { span, .. }
        | Stmt::TryCatch { span, .. }
        | Stmt::DebugPrint { span, .. } => *span,
    }
}

fn index_labels(statements: &[Stmt]) -> HashMap<String, usize> {
    let mut labels = HashMap::new();
    for (index, stmt) in statements.iter().enumerate() {
        if let Stmt::Label { name, .. } = stmt {
            labels.insert(super::values::key(name), index + 1);
        }
    }
    labels
}

fn index_line_numbers(statements: &[Stmt]) -> HashMap<usize, i64> {
    let mut line_numbers = HashMap::new();
    for (index, window) in statements.windows(2).enumerate() {
        let Stmt::Label { name, span } = &window[0] else {
            continue;
        };
        let Ok(number) = name.parse::<i64>() else {
            continue;
        };
        if span.start.line == stmt_span(&window[1]).start.line {
            line_numbers.insert(index + 1, number);
        }
    }
    line_numbers
}
