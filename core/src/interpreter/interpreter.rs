use std::collections::HashMap;

use crate::runtime::Diagnostic;
use crate::{Function, Procedure, Program};

use super::records::RuntimeType;
use super::values::key;
use super::{ControlFlow, Frame, RuntimeClass, RuntimeEnum};

#[derive(Debug, Default)]
pub struct Interpreter {
    pub(crate) types: HashMap<String, RuntimeType>,
    pub(crate) enums: HashMap<String, RuntimeEnum>,
    pub(crate) enum_members: HashMap<String, i64>,
    pub(crate) classes: HashMap<String, RuntimeClass>,
    pub(crate) procedures: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) output: Vec<String>,
    pub(crate) option_base: i64,
    pub(crate) option_compare: crate::OptionCompare,
    pub(crate) call_stack: Vec<String>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

impl Interpreter {
    pub fn new() -> Self {
        Self {
            option_compare: crate::OptionCompare::Binary,
            ..Self::default()
        }
    }

    pub fn run(mut self, program: &Program) -> Result<Vec<String>, Diagnostic> {
        self.option_base = program.option_base;
        self.option_compare = program.option_compare;
        for type_decl in &program.types {
            self.types
                .insert(key(&type_decl.name), RuntimeType::from(type_decl));
        }
        for enum_decl in &program.enums {
            let mut members = HashMap::new();
            let mut previous = -1;
            for member in &enum_decl.members {
                let value = if let Some(expr) = &member.value {
                    self.eval_enum_const_expr(expr, &members)?
                } else {
                    previous + 1
                };
                previous = value;
                members.insert(key(&member.name), value);
                self.enum_members.insert(key(&member.name), value);
            }
            self.enums.insert(
                key(&enum_decl.name),
                RuntimeEnum {
                    name: enum_decl.name.clone(),
                    members,
                },
            );
        }
        for class_decl in &program.classes {
            self.classes
                .insert(key(&class_decl.name), RuntimeClass::from(class_decl));
        }
        for procedure in &program.procedures {
            self.procedures
                .insert(key(&procedure.name), procedure.clone());
        }
        for function in &program.functions {
            self.functions.insert(key(&function.name), function.clone());
        }

        let mut frame = Frame::default();
        for var in &program.module_vars {
            frame.declare_module(
                &var.name,
                var.ty.clone(),
                var.array.clone(),
                self.option_base,
                false,
                None,
                var.span,
                &self.types,
                &self.enums,
            )?;
        }
        for const_decl in &program.module_consts {
            let value = self.eval_expr(&const_decl.value, &mut frame)?;
            let ty = const_decl.ty.clone().unwrap_or_else(|| value.type_name());
            frame.declare_module(
                &const_decl.name,
                ty,
                None,
                self.option_base,
                true,
                Some(value),
                const_decl.span,
                &self.types,
                &self.enums,
            )?;
        }

        let Some(main) = program
            .procedures
            .iter()
            .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
        else {
            return Err(Diagnostic::new("Program must contain Sub Main()", None));
        };

        match self.exec_block(&main.body, &mut frame)? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(self.output),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                "Exit Function is only valid inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFor | ControlFlow::ExitWhile | ControlFlow::ExitDo => Err(
                Diagnostic::new("Exit statement escaped its block", Some(main.span)),
            ),
        }
    }
}

pub fn run(program: &Program) -> Result<Vec<String>, Diagnostic> {
    Interpreter::new().run(program)
}

impl Interpreter {
    pub(crate) fn with_stack_context(&self, diagnostic: Diagnostic) -> Diagnostic {
        if self.call_stack.is_empty() {
            diagnostic
        } else {
            diagnostic.with_note(format!("while executing {}", self.call_stack.join(" -> ")))
        }
    }

    fn eval_enum_const_expr(
        &self,
        expr: &crate::Expr,
        members: &HashMap<String, i64>,
    ) -> Result<i64, Diagnostic> {
        use crate::{BinaryOp, ExprKind, UnaryOp};
        match &expr.kind {
            ExprKind::Integer(value) => Ok(*value),
            ExprKind::Variable(name) => members.get(&key(name)).copied().ok_or_else(|| {
                Diagnostic::new(
                    format!("Enum member '{}' is not defined", name),
                    Some(expr.span),
                )
            }),
            ExprKind::Unary {
                op: UnaryOp::Negate,
                expr,
            } => Ok(-self.eval_enum_const_expr(expr, members)?),
            ExprKind::Binary { left, op, right } => {
                let left = self.eval_enum_const_expr(left, members)?;
                let right = self.eval_enum_const_expr(right, members)?;
                match op {
                    BinaryOp::Add => Ok(left + right),
                    BinaryOp::Subtract => Ok(left - right),
                    BinaryOp::Multiply => Ok(left * right),
                    BinaryOp::Divide => {
                        if right == 0 {
                            Err(Diagnostic::new("Division by zero", Some(expr.span)))
                        } else {
                            Ok(left / right)
                        }
                    }
                    _ => Err(Diagnostic::new(
                        "Enum value expression must be numeric",
                        Some(expr.span),
                    )),
                }
            }
            _ => Err(Diagnostic::new(
                "Enum value expression must be numeric",
                Some(expr.span),
            )),
        }
    }
}
