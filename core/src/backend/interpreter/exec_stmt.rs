use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::{Diagnostic, RuntimeErrorInfo, TypeName, Value};
use crate::{
    AssignTarget, CaseItem, DoLoopCondition, ExitTarget, OnErrorMode, ReDimTarget, ResumeTarget,
    Stmt, UsingResource,
};

use super::values::key;
use super::{ControlFlow, Frame, Interpreter};
use crate::runtime::compare::{
    RuntimeCompareOp, RuntimeOptionCompare, compare_case_values, values_equal,
};

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
            self.temporary_strings.clear();
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
        if self.terminated {
            return Ok(ControlFlow::Terminate);
        }
        match stmt {
            Stmt::Dim {
                name,
                ty,
                array,
                as_new,
                new_args,
                initializer,
                span,
            } => {
                self.exec_variable_declaration(
                    name,
                    ty,
                    array,
                    *as_new,
                    new_args,
                    initializer,
                    frame,
                    *span,
                )?;
                Ok(ControlFlow::Continue)
            }
            Stmt::DimMany { decls, .. } => {
                for decl in decls {
                    self.exec_variable_declaration(
                        &decl.name,
                        &decl.ty,
                        &decl.array,
                        decl.as_new,
                        &decl.new_args,
                        &decl.initializer,
                        frame,
                        decl.span,
                    )?;
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::Static {
                name,
                ty,
                array,
                as_new,
                new_args,
                initializer,
                span,
            } => {
                self.exec_static_declaration(
                    name,
                    ty,
                    array,
                    *as_new,
                    new_args,
                    initializer,
                    frame,
                    *span,
                )?;
                Ok(ControlFlow::Continue)
            }
            Stmt::StaticMany { decls, .. } => {
                for decl in decls {
                    self.exec_static_declaration(
                        &decl.name,
                        &decl.ty,
                        &decl.array,
                        decl.as_new,
                        &decl.new_args,
                        &decl.initializer,
                        frame,
                        decl.span,
                    )?;
                }
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
            Stmt::ConstMany { consts, .. } => {
                for const_decl in consts {
                    let value = self.eval_expr(&const_decl.value, frame)?;
                    let ty = const_decl.ty.clone().unwrap_or_else(|| value.type_name());
                    let ty = self.resolve_type_name(&ty, frame, const_decl.span)?;
                    frame.declare_const(&const_decl.name, ty, value, const_decl.span)?;
                }
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
            Stmt::ConsoleCall { method, args, .. } => {
                if let Some(flow) = super::builtins::dispatch_stmt(
                    self,
                    "Console",
                    method,
                    args,
                    frame,
                    stmt_span(stmt),
                )? {
                    return Ok(flow);
                }
                Ok(ControlFlow::Continue)
            }
            Stmt::End { .. } => {
                self.terminated = true;
                Ok(ControlFlow::Terminate)
            }
            Stmt::SubCall { name, args, span } => {
                if let Some(flow) =
                    super::builtins::dispatch_stmt(self, "VBA", name, args, frame, *span)?
                {
                    return Ok(flow);
                }

                if let Ok(me) = frame.get("me", *span)
                    && let Value::Object(ref obj) = me
                {
                    let class_name = obj.borrow().class_name.clone();
                    if let Some(class) = self.classes.get(&super::values::key(&class_name))
                        && class.subs.contains_key(&super::values::key(name))
                    {
                        self.call_method_sub(me, name, args, frame, *span)?;
                        return Ok(ControlFlow::Continue);
                    }
                }

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
                    && let Some(flow) =
                        super::builtins::dispatch_stmt(self, name, method, args, frame, *span)?
                {
                    return Ok(flow);
                }
                if let crate::ExprKind::Variable(module_name) = &object.kind
                    && self
                        .resolve_module_qualifier(module_name, frame, *span)
                        .is_ok()
                {
                    self.call_module_sub(module_name, method, args, frame, *span)?;
                    return Ok(ControlFlow::Continue);
                }
                if let crate::ExprKind::Variable(class_name) = &object.kind
                    && !frame.has_variable(class_name)
                    && self.classes.contains_key(&super::values::key(class_name))
                {
                    self.call_shared_sub(class_name, method, args, frame, *span)?;
                    return Ok(ControlFlow::Continue);
                }
                if let crate::ExprKind::Variable(name) = &object.kind
                    && let Ok(variable) = frame.variable(name, object.span)
                    && matches!(
                        &*variable.borrow(),
                        Value::Record(_) | Value::BoxedRecord(_, _)
                    )
                {
                    self.call_record_sub_variable(variable, method, args, frame, *span)?;
                    return Ok(ControlFlow::Continue);
                }
                if matches!(object.kind, crate::ExprKind::Me)
                    && let Ok(variable) = frame.variable("me", object.span)
                    && matches!(
                        &*variable.borrow(),
                        Value::Record(_) | Value::BoxedRecord(_, _)
                    )
                {
                    self.call_record_sub_variable(variable, method, args, frame, *span)?;
                    return Ok(ControlFlow::Continue);
                }
                if matches!(object.kind, crate::ExprKind::MyBase) {
                    let object = self.eval_expr(object, frame)?;
                    let Value::Object(instance) = object else {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            "MyBase is only valid inside class methods",
                            Some(*span),
                        ));
                    };
                    let current_class = frame.current_class_name().ok_or_else(|| {
                        Diagnostic::new(
                            crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                            "MyBase is only valid inside class methods",
                            Some(*span),
                        )
                    })?;
                    let base_class = self
                        .classes
                        .get(&super::values::key(&current_class))
                        .and_then(|class| class.base_class.as_ref())
                        .map(crate::runtime::TypeName::display_name)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                                format!("Class '{}' has no base class", current_class),
                                Some(*span),
                            )
                        })?;
                    self.call_method_sub_on_runtime_class(
                        instance,
                        &base_class,
                        method,
                        args,
                        frame,
                        *span,
                    )?;
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
            Stmt::Yield { expr, .. } => {
                let value = self.eval_expr(expr, frame)?;
                frame.yield_value(value);
                Ok(ControlFlow::Continue)
            }
            Stmt::Throw { expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                let message = value.to_string();
                Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::RUNTIME_ERROR,
                    message,
                    Some(*span),
                ))
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

                    let old = frame.assign(variable, Value::Int64(current), *span)?;
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
                let values = super::arrays::enumerable_values(self, iterable, frame, *span)?;
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
                target,
                dims,
                preserve,
                span,
            } => {
                let mut new_bounds = Vec::new();
                for (lower_expr, upper_expr) in dims {
                    let lower = if let Some(lower_expr) = lower_expr {
                        self.eval_integer_expr(
                            lower_expr,
                            frame,
                            "ReDim lower bound must be Integer",
                        )?
                    } else {
                        self.option_base
                    };
                    let upper = self.eval_integer_expr(
                        upper_expr,
                        frame,
                        "ReDim upper bound must be Integer",
                    )?;
                    if upper < lower {
                        return Err(Diagnostic::new(
                            crate::runtime::DiagnosticCode::ARRAY,
                            "Array upper bound must be greater than or equal to lower bound",
                            Some(*span),
                        ));
                    }
                    new_bounds.push(crate::runtime::ArrayBound { lower, upper });
                }
                self.redim_target(target, new_bounds, *preserve, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Erase { target, span } => {
                match target {
                    ReDimTarget::Variable { name, .. } => {
                        frame.erase_array(name, *span, self)?;
                    }
                    ReDimTarget::Member { object, field, .. } => {
                        let obj_value = self.eval_expr(object, frame)?;
                        self.erase_member_array(&obj_value, field, *span, frame)?;
                    }
                }
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
            Stmt::Using {
                resource,
                body,
                span,
            } => self.exec_using(resource, body, frame, *span),
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
                            self,
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
            Stmt::OpenFile {
                path,
                mode,
                access,
                lock,
                shared,
                number,
                record_len,
                span,
            } => {
                self.open_file(
                    super::file_io::OpenFileRequest {
                        path,
                        mode: *mode,
                        access: *access,
                        lock: *lock,
                        shared: *shared,
                        number,
                        record_len: record_len.as_ref(),
                        span: *span,
                    },
                    frame,
                )?;
                Ok(ControlFlow::Continue)
            }
            Stmt::CloseFile { numbers, .. } => {
                self.close_files(numbers, frame)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::LineInput {
                number,
                target,
                span,
            } => {
                self.line_input_file(number, target, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::InputFile {
                number,
                targets,
                span,
            } => {
                self.input_file(number, targets, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::PrintFile {
                number,
                items,
                trailing,
                span,
            } => {
                self.print_file(number, items, *trailing, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::WriteFile { number, args, span } => {
                self.write_file(number, args, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::GetFile {
                number,
                position,
                target,
                span,
            } => {
                self.get_file(number, position.as_ref(), target, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::PutFile {
                number,
                position,
                expr,
                span,
            } => {
                self.put_file(number, position.as_ref(), expr, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::SeekFile {
                number,
                position,
                span,
            } => {
                self.seek_file_statement(number, position, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::NameFile {
                old_path,
                new_path,
                span,
            } => {
                self.name_file(old_path, new_path, frame, *span)?;
                Ok(ControlFlow::Continue)
            }
        }
    }

    fn exec_using(
        &mut self,
        resource: &UsingResource,
        body: &[Stmt],
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<ControlFlow, Diagnostic> {
        let (resource_value, declared_name) = match resource {
            UsingResource::Declaration(decl) => {
                self.exec_variable_declaration(
                    &decl.name,
                    &decl.ty,
                    &decl.array,
                    decl.as_new,
                    &decl.new_args,
                    &decl.initializer,
                    frame,
                    decl.span,
                )?;
                (frame.get(&decl.name, decl.span)?, Some(decl.name.as_str()))
            }
            UsingResource::Target(expr) => (self.eval_expr(expr, frame)?, None),
        };

        let body_result = self.exec_block(body, frame);
        let dispose_result = self.call_dispose(resource_value, frame, span);
        let remove_result = if let Some(name) = declared_name {
            if let Some(variable) = frame.remove_variable(name) {
                let value = variable.borrow().clone();
                drop(variable);
                self.maybe_terminate(value, span)
            } else {
                Ok(())
            }
        } else {
            Ok(())
        };

        match (body_result, dispose_result, remove_result) {
            (Err(body_error), Err(dispose_error), _) => Err(body_error.with_related(dispose_error)),
            (Err(body_error), Ok(()), Err(terminate_error)) => {
                Err(body_error.with_related(terminate_error))
            }
            (Err(body_error), Ok(()), Ok(())) => Err(body_error),
            (Ok(_), Err(dispose_error), _) => Err(dispose_error),
            (Ok(_), Ok(()), Err(terminate_error)) => Err(terminate_error),
            (Ok(flow), Ok(()), Ok(())) => Ok(flow),
        }
    }

    fn call_dispose(
        &mut self,
        value: Value,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        match &value {
            Value::Object(instance) => {
                let class_name = instance.borrow().class_name.clone();
                let class = self.classes.get(&key(&class_name)).ok_or_else(|| {
                    Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!("Using target class '{}' is not disposable", class_name),
                        Some(span),
                    )
                })?;
                let Some(dispose) = class.subs.get("dispose") else {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::MEMBER_ACCESS,
                        format!("Using target class '{}' has no Dispose method", class.name),
                        Some(span),
                    ));
                };
                if !dispose.params.is_empty() {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "Dispose method used by Using must be parameterless",
                        Some(span),
                    ));
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                    "Using target must be a class instance with a parameterless Dispose method",
                    Some(span),
                ));
            }
        }
        self.call_method_sub_values(value, "Dispose", &[], frame, span)
    }

    #[allow(clippy::too_many_arguments)]
    fn exec_variable_declaration(
        &mut self,
        name: &str,
        ty: &Option<crate::runtime::TypeName>,
        array: &Option<crate::ArrayDecl>,
        as_new: bool,
        new_args: &[crate::Expr],
        initializer: &Option<crate::Expr>,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        let initial_value = if let Some(initializer) = initializer {
            Some(self.eval_expr(initializer, frame)?)
        } else {
            None
        };
        let ty = if let Some(ty) = ty {
            self.resolve_type_name(ty, frame, span)?
        } else {
            initial_value
                .as_ref()
                .map(Value::type_name)
                .unwrap_or(crate::runtime::TypeName::Variant)
        };
        let ty_for_new = ty.clone();
        frame.declare(name, ty, array.clone(), self.option_base, span, self)?;
        if as_new {
            match &ty_for_new {
                TypeName::User(_) | TypeName::GenericInstance { .. } => {
                    let value = self.new_object(&ty_for_new, new_args, frame, span)?;
                    let _ = frame.assign(name, value, span)?;
                }
                _ => {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "As New requires a class type",
                        Some(span),
                    ));
                }
            }
        }
        if let Some(value) = initial_value {
            let init_span = initializer
                .as_ref()
                .expect("value came from initializer")
                .span;
            let _ = frame.assign(name, value, init_span)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn exec_static_declaration(
        &mut self,
        name: &str,
        ty: &Option<crate::runtime::TypeName>,
        array: &Option<crate::ArrayDecl>,
        as_new: bool,
        _new_args: &[crate::Expr],
        initializer: &Option<crate::Expr>,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        if as_new {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Static As New is not supported",
                Some(span),
            ));
        }
        let initial_value = if let Some(initializer) = initializer {
            Some(self.eval_expr(initializer, frame)?)
        } else {
            None
        };
        let ty = if let Some(ty) = ty {
            self.resolve_type_name(ty, frame, span)?
        } else {
            initial_value
                .as_ref()
                .map(Value::type_name)
                .unwrap_or(crate::runtime::TypeName::Variant)
        };
        let scope = self
            .scope_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "<module>".to_string());
        let mut static_frame = self.static_frames.remove(&scope).unwrap_or_default();
        let already_declared = static_frame.has_variable(name);
        frame.declare_static(
            name,
            ty,
            array.clone(),
            self.option_base,
            span,
            self,
            &mut static_frame,
        )?;
        if !already_declared && let Some(value) = initial_value {
            let init_span = initializer
                .as_ref()
                .expect("value came from initializer")
                .span;
            let _ = frame.assign(name, value, init_span)?;
        }
        self.static_frames.insert(scope, static_frame);
        Ok(())
    }

    fn create_error_object(&self, diagnostic: &Diagnostic) -> Value {
        let mut fields = HashMap::new();
        if let Some(info) = &diagnostic.runtime_error {
            fields.insert(key("Number"), Value::Int64(info.number));
            fields.insert(key("Message"), Value::String(info.description.clone()));
            fields.insert(key("Description"), Value::String(info.description.clone()));
            fields.insert(key("Source"), Value::String(info.source.clone()));
            fields.insert(key("HelpFile"), Value::String(info.help_file.clone()));
            fields.insert(key("HelpContext"), Value::Int64(info.help_context));
        } else {
            fields.insert(key("Number"), Value::Int64(1));
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
            fields.insert(key("HelpContext"), Value::Int64(0));
        }

        Value::Object(Rc::new(RefCell::new(crate::runtime::ObjectValue {
            class_name: "Error".to_string(),
            fields,
            event_bindings: Vec::new(),
            terminated: false,
        })))
    }

    pub(crate) fn assign_target(
        &mut self,
        target: &AssignTarget,
        value: Value,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        match target {
            AssignTarget::Variable { name, .. } => {
                if let Some(_slot) = frame.get_return_slot(&format!("__return_{}", name)) {
                    // This logic will be used when `name` is the function name.
                    // However, we need a way to detect if it's the current function.
                    // For now, assume if it's in the return_slots, it's a return assignment.
                    frame.set_return_slot(format!("__return_{}", name), value);
                    return Ok(());
                }

                if let Ok(owner_variable) = frame.variable("me", span) {
                    let is_record_field = {
                        let owner = owner_variable.borrow();
                        matches!(
                            &*owner,
                            Value::Record(record)
                                if record.fields.contains_key(&super::values::key(name))
                        )
                    };
                    if is_record_field {
                        return self.assign_member_to_variable(owner_variable, name, value, span);
                    }
                }
                if frame.has_variable(name) {
                    let old = frame.assign(name, value, span)?;
                    self.maybe_terminate(old, span)
                } else {
                    let owner_variable = frame.variable("me", span)?;
                    if matches!(&*owner_variable.borrow(), Value::Record(_)) {
                        self.assign_member_to_variable(owner_variable, name, value, span)
                    } else {
                        let owner = owner_variable.borrow().clone();
                        self.assign_bare_class_field(owner, name, value, span)
                    }
                }
            }
            AssignTarget::ArrayElement { name, indices, .. } => {
                let mut index_values = Vec::new();
                for index_expr in indices {
                    index_values.push(self.eval_expr(index_expr, frame)?);
                }

                let old = if frame.has_variable(name) {
                    let target = frame.get(name, span)?;
                    if let Value::Object(ref object) = target {
                        let class_name = object.borrow().class_name.clone();
                        if let Some(default_member) = self
                            .classes
                            .get(&super::values::key(&class_name))
                            .and_then(|class| class.default_member.clone())
                        {
                            index_values.push(value);
                            self.call_property_set_values(
                                target,
                                &default_member,
                                &index_values,
                                span,
                            )?;
                            return Ok(());
                        }
                    }
                    if let Value::ComObject(ref com_obj) = target {
                        let mut property_args = index_values;
                        property_args.push(value);
                        crate::runtime::com::invoke_default_com(
                            com_obj,
                            &property_args,
                            4, // DISPATCH_PROPERTYPUT
                            span,
                        )?;
                        return Ok(());
                    }

                    let mut dims = Vec::new();
                    for (index_expr, index_value) in indices.iter().zip(index_values.iter()) {
                        dims.push(match index_value {
                            Value::Byte(value) => i64::from(*value),
                            Value::Int16(value) => i64::from(*value),
                            Value::Int32(value) => i64::from(*value),
                            Value::Int64(value) => *value,
                            _ => {
                                return Err(Diagnostic::new(
                                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                    "Array index must be Integer",
                                    Some(index_expr.span),
                                ));
                            }
                        });
                    }
                    frame.assign_array_element(name, &dims, value, span)?
                } else {
                    let mut dims = Vec::new();
                    for (index_expr, index_value) in indices.iter().zip(index_values.iter()) {
                        dims.push(match index_value {
                            Value::Byte(value) => i64::from(*value),
                            Value::Int16(value) => i64::from(*value),
                            Value::Int32(value) => i64::from(*value),
                            Value::Int64(value) => *value,
                            _ => {
                                return Err(Diagnostic::new(
                                    crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                                    "Array index must be Integer",
                                    Some(index_expr.span),
                                ));
                            }
                        });
                    }
                    let owner = frame.get("me", span)?;
                    self.assign_bare_class_field_array_element(owner, name, &dims, value, span)?
                };
                self.maybe_terminate(old, span)
            }
            AssignTarget::Member { object, field, .. } => {
                self.assign_member(object, field, value, frame, span)
            }
            AssignTarget::MemberArrayElement {
                object,
                field,
                indices,
                ..
            } => {
                let mut index_values = Vec::new();
                for index_expr in indices {
                    index_values.push(self.eval_expr(index_expr, frame)?);
                }
                self.assign_member_element(object, field, index_values, value, frame, span)
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
                let runtime_compare = match self.option_compare {
                    crate::OptionCompare::Binary => RuntimeOptionCompare::Binary,
                    crate::OptionCompare::Text => RuntimeOptionCompare::Text,
                };
                Ok(values_equal(subject, &value, runtime_compare))
            }
            CaseItem::Range { start, end } => {
                let start_value = self.eval_expr(start, frame)?;
                let end_value = self.eval_expr(end, frame)?;
                let runtime_compare = match self.option_compare {
                    crate::OptionCompare::Binary => RuntimeOptionCompare::Binary,
                    crate::OptionCompare::Text => RuntimeOptionCompare::Text,
                };
                let lower = compare_case_values(
                    subject.clone(),
                    RuntimeCompareOp::GreaterEqual,
                    start_value,
                    runtime_compare,
                    start.span,
                )?;
                let upper = compare_case_values(
                    subject.clone(),
                    RuntimeCompareOp::LessEqual,
                    end_value,
                    runtime_compare,
                    end.span,
                )?;
                Ok(lower.is_truthy() && upper.is_truthy())
            }
            CaseItem::Compare { op, expr } => {
                let value = self.eval_expr(expr, frame)?;
                let runtime_op = match op {
                    crate::CaseCompareOp::Equal => RuntimeCompareOp::Equal,
                    crate::CaseCompareOp::NotEqual => RuntimeCompareOp::NotEqual,
                    crate::CaseCompareOp::Less => RuntimeCompareOp::Less,
                    crate::CaseCompareOp::Greater => RuntimeCompareOp::Greater,
                    crate::CaseCompareOp::LessEqual => RuntimeCompareOp::LessEqual,
                    crate::CaseCompareOp::GreaterEqual => RuntimeCompareOp::GreaterEqual,
                };
                let runtime_compare = match self.option_compare {
                    crate::OptionCompare::Binary => RuntimeOptionCompare::Binary,
                    crate::OptionCompare::Text => RuntimeOptionCompare::Text,
                };
                Ok(compare_case_values(
                    subject.clone(),
                    runtime_op,
                    value,
                    runtime_compare,
                    expr.span,
                )?
                .is_truthy())
            }
        }
    }

    pub(crate) fn err_raise(
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
        | Stmt::DimMany { span, .. }
        | Stmt::Static { span, .. }
        | Stmt::StaticMany { span, .. }
        | Stmt::Const { span, .. }
        | Stmt::ConstMany { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::SetAssign { span, .. }
        | Stmt::ConsoleCall { span, .. }
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
        | Stmt::Using { span, .. }
        | Stmt::Exit { span, .. }
        | Stmt::TryCatch { span, .. }
        | Stmt::DebugPrint { span, .. }
        | Stmt::OpenFile { span, .. }
        | Stmt::CloseFile { span, .. }
        | Stmt::LineInput { span, .. }
        | Stmt::InputFile { span, .. }
        | Stmt::PrintFile { span, .. }
        | Stmt::WriteFile { span, .. }
        | Stmt::GetFile { span, .. }
        | Stmt::PutFile { span, .. }
        | Stmt::SeekFile { span, .. }
        | Stmt::NameFile { span, .. }
        | Stmt::Yield { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::End { span } => *span,
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
