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
    pub(crate) scope_stack: Vec<String>,
    pub(crate) static_frames: HashMap<String, Frame>,
    pub(crate) err_number: i64,
    pub(crate) err_description: String,
    pub(crate) err_source: String,
    pub(crate) err_help_file: String,
    pub(crate) err_help_context: i64,
    pub(crate) erl: i64,
    pub(crate) module_frames: HashMap<String, Frame>,
    pub(crate) module_imports: HashMap<String, Vec<RuntimeImport>>,
    pub(crate) function_modules: HashMap<String, Vec<String>>,
    pub(crate) sub_modules: HashMap<String, Vec<String>>,
    pub(crate) public_values: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeImport {
    pub(crate) qualifier: String,
    pub(crate) module: String,
}

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
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Program must contain Sub Main()",
                None,
            ));
        };

        self.scope_stack.push(format!("Sub {}", main.name));
        let result = self.exec_block(&main.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(self.output),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit statement escaped its block",
                Some(main.span),
            )),
        }
    }

    pub fn run_project(
        mut self,
        project: &crate::modules::Project,
    ) -> Result<Vec<String>, Diagnostic> {
        self.load_project_runtime(project)?;
        let entry = &project.modules[project.entry];
        let Some(main) = entry
            .program
            .procedures
            .iter()
            .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
            .cloned()
        else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Program must contain Sub Main()",
                None,
            ));
        };
        let module_key = super::values::key(&entry.name);
        let mut frame = self
            .module_frames
            .get(&module_key)
            .cloned()
            .expect("module frame initialized");
        frame.set_module_key(module_key);
        self.scope_stack.push(format!("Sub {}", main.name));
        let result = self.exec_block(&main.body, &mut frame);
        self.scope_stack.pop();
        match result? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(self.output),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Return is only allowed inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit Function is only valid inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFor
            | ControlFlow::ExitWhile
            | ControlFlow::ExitDo
            | ControlFlow::GoTo(_)
            | ControlFlow::Resume(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::CONTROL_FLOW,
                "Exit statement escaped its block",
                Some(main.span),
            )),
        }
    }

    fn load_project_runtime(
        &mut self,
        project: &crate::modules::Project,
    ) -> Result<(), Diagnostic> {
        let entry_key = super::values::key(&project.modules[project.entry].name);
        for module in &project.modules {
            self.option_base = module.program.option_base;
            self.option_compare = module.program.option_compare;
            let module_key = super::values::key(&module.name);
            for type_decl in &module.program.types {
                if module_key == entry_key || crate::modules::is_public(type_decl.visibility) {
                    self.types.insert(
                        super::values::key(&type_decl.name),
                        RuntimeType::from(type_decl),
                    );
                }
            }
            for enum_decl in &module.program.enums {
                if module_key != entry_key && !crate::modules::is_public(enum_decl.visibility) {
                    continue;
                }
                let mut members = HashMap::new();
                let mut previous = -1;
                for member in &enum_decl.members {
                    let value = if let Some(expr) = &member.value {
                        self.eval_enum_const_expr(expr, &members)?
                    } else {
                        previous + 1
                    };
                    previous = value;
                    members.insert(super::values::key(&member.name), value);
                    self.enum_members
                        .insert(super::values::key(&member.name), value);
                }
                self.enums.insert(
                    super::values::key(&enum_decl.name),
                    RuntimeEnum {
                        name: enum_decl.name.clone(),
                        members,
                    },
                );
            }
            for class_decl in &module.program.classes {
                if module_key == entry_key || crate::modules::is_public(class_decl.visibility) {
                    self.classes.insert(
                        super::values::key(&class_decl.name),
                        RuntimeClass::from(class_decl),
                    );
                }
            }
            for procedure in &module.program.procedures {
                self.procedures.insert(
                    format!("{}::{}", module_key, super::values::key(&procedure.name)),
                    procedure.clone(),
                );
                if module_key == entry_key || crate::modules::is_public(procedure.visibility) {
                    self.sub_modules
                        .entry(super::values::key(&procedure.name))
                        .or_default()
                        .push(module_key.clone());
                }
            }
            for function in &module.program.functions {
                self.functions.insert(
                    format!("{}::{}", module_key, super::values::key(&function.name)),
                    function.clone(),
                );
                if module_key == entry_key || crate::modules::is_public(function.visibility) {
                    self.function_modules
                        .entry(super::values::key(&function.name))
                        .or_default()
                        .push(module_key.clone());
                }
            }
        }
        for module in &project.modules {
            let module_key = super::values::key(&module.name);
            self.module_imports.insert(
                module_key.clone(),
                module
                    .imports
                    .iter()
                    .map(|import| RuntimeImport {
                        qualifier: import.qualifier.clone(),
                        module: super::values::key(&project.modules[import.module].name),
                    })
                    .collect(),
            );
            let mut frame = Frame::default();
            frame.set_module_key(module_key.clone());
            let mut public_values = Vec::new();
            for var in &module.program.module_vars {
                if crate::modules::is_public(var.visibility) {
                    public_values.push(super::values::key(&var.name));
                }
                frame.declare_module(
                    &var.name,
                    var.ty.clone(),
                    var.array.clone(),
                    module.program.option_base,
                    false,
                    None,
                    var.span,
                    &self.types,
                    &self.enums,
                )?;
            }
            for const_decl in &module.program.module_consts {
                if crate::modules::is_public(const_decl.visibility) {
                    public_values.push(super::values::key(&const_decl.name));
                }
                let value = self.eval_expr(&const_decl.value, &mut frame)?;
                let ty = const_decl.ty.clone().unwrap_or_else(|| value.type_name());
                frame.declare_module(
                    &const_decl.name,
                    ty,
                    None,
                    module.program.option_base,
                    true,
                    Some(value),
                    const_decl.span,
                    &self.types,
                    &self.enums,
                )?;
            }
            self.public_values.insert(module_key.clone(), public_values);
            self.module_frames.insert(module_key, frame);
        }
        Ok(())
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

    pub(crate) fn set_err(&mut self, diagnostic: &Diagnostic, erl: i64) {
        if let Some(info) = &diagnostic.runtime_error {
            self.err_number = info.number;
            self.err_description = info.description.clone();
            self.err_source = info.source.clone();
            self.err_help_file = info.help_file.clone();
            self.err_help_context = info.help_context;
        } else {
            self.err_number = 1;
            self.err_description = diagnostic.message.to_string();
            self.err_source = "Valo.Runtime".to_string();
            self.err_help_file.clear();
            self.err_help_context = 0;
        }
        self.erl = erl;
    }

    pub(crate) fn clear_err(&mut self) {
        self.err_number = 0;
        self.err_description.clear();
        self.err_source.clear();
        self.err_help_file.clear();
        self.err_help_context = 0;
        self.erl = 0;
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
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
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
                            Err(Diagnostic::new(
                                crate::runtime::DiagnosticCode::GENERIC,
                                "Division by zero",
                                Some(expr.span),
                            ))
                        } else {
                            Ok(left / right)
                        }
                    }
                    _ => Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                        "Enum value expression must be numeric",
                        Some(expr.span),
                    )),
                }
            }
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "Enum value expression must be numeric",
                Some(expr.span),
            )),
        }
    }
}
