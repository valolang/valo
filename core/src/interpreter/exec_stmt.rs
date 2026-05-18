use crate::runtime::{Diagnostic, Value};
use crate::{
    AssignTarget, CaseCompareOp, CaseItem, DoLoopCondition, ExitTarget, OnErrorMode, Stmt,
};
use std::collections::HashMap;

use super::values::{compare_case_values, values_equal};
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn exec_block(
        &mut self,
        statements: &[Stmt],
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        let labels = index_labels(statements);
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
                Ok(flow) => return Ok(flow),
                Err(error) if frame.resume_next() => {
                    self.set_err(&error);
                    ip += 1;
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
                span,
            } => {
                frame.declare(
                    name,
                    ty.clone(),
                    array.clone(),
                    self.option_base,
                    *span,
                    &self.types,
                    &self.enums,
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
                    parts.push(self.eval_expr(arg, frame)?.to_output_string());
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
                    && name.eq_ignore_ascii_case("Err")
                    && method.eq_ignore_ascii_case("Clear")
                    && args.is_empty()
                {
                    self.clear_err();
                    return Ok(ControlFlow::Continue);
                }
                let object = self.eval_expr(object, frame)?;
                self.call_method_sub(object, method, args, frame, *span)?;
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
                    return Err(Diagnostic::new("For Step cannot be zero", Some(*span)));
                }

                loop {
                    if (step > 0 && current > end) || (step < 0 && current < end) {
                        break;
                    }

                    frame.assign(variable, Value::Integer(current), *span)?;
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
                    frame.assign(variable, value, *span)?;
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
                upper_bound,
                preserve,
                span,
            } => {
                let upper_bound = self.eval_integer_expr(
                    upper_bound,
                    frame,
                    "ReDim upper bound must be Integer",
                )?;
                frame.redim_array(
                    name,
                    upper_bound,
                    self.option_base,
                    *preserve,
                    &self.types,
                    &self.enums,
                    *span,
                )?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Label { .. } => Ok(ControlFlow::Continue),
            Stmt::GoTo { label, .. } => Ok(ControlFlow::GoTo(label.clone())),
            Stmt::OnError { mode, .. } => {
                match mode {
                    OnErrorMode::ResumeNext => frame.set_resume_next(true),
                    OnErrorMode::GoToZero => frame.set_resume_next(false),
                }
                Ok(ControlFlow::Continue)
            }
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
        }
    }

    fn assign_target(
        &mut self,
        target: &AssignTarget,
        value: Value,
        frame: &mut Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        match target {
            AssignTarget::Variable { name, .. } => frame.assign(name, value, span),
            AssignTarget::ArrayElement { name, index, .. } => {
                let index = self.eval_integer_expr(index, frame, "Array index must be Integer")?;
                frame.assign_array_element(name, index, value, span)
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
