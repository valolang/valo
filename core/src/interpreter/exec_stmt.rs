use crate::runtime::{Diagnostic, Value};
use crate::{DoLoopCondition, ExitTarget, Stmt};

use super::values::values_equal;
use super::{ControlFlow, Frame, Interpreter};

impl Interpreter {
    pub(crate) fn exec_block(
        &mut self,
        statements: &[Stmt],
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        for stmt in statements {
            match self.exec_stmt(stmt, frame)? {
                ControlFlow::Continue => {}
                flow => return Ok(flow),
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
                array_size,
                span,
            } => {
                frame.declare(name, ty.clone(), *array_size, *span, &self.types)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::Assign { name, expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                frame.assign(name, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::SetAssign { name, expr, span } => {
                let value = self.eval_expr(expr, frame)?;
                frame.assign(name, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::ArrayAssign {
                name,
                index,
                expr,
                span,
            } => {
                let index = self.eval_integer_expr(index, frame, "Array index must be Integer")?;
                let value = self.eval_expr(expr, frame)?;
                frame.assign_array_element(name, index, value, *span)?;
                Ok(ControlFlow::Continue)
            }
            Stmt::MemberAssign {
                target,
                field,
                expr,
                span,
            } => {
                let value = self.eval_expr(expr, frame)?;
                self.assign_member(target, field, value, frame, *span)?;
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
                    for case_value in &branch.values {
                        let case_value = self.eval_expr(case_value, frame)?;
                        if values_equal(&subject, &case_value) {
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
            Stmt::Exit { target, .. } => match target {
                ExitTarget::Sub => Ok(ControlFlow::ExitSub),
                ExitTarget::Function => Ok(ControlFlow::ExitFunction),
                ExitTarget::For => Ok(ControlFlow::ExitFor),
                ExitTarget::While => Ok(ControlFlow::ExitWhile),
                ExitTarget::Do => Ok(ControlFlow::ExitDo),
            },
        }
    }
}
