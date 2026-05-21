use rand::SeedableRng;
use rand_pcg::Pcg64;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::runtime::{Diagnostic, TypeName, Value};
use crate::{DeclareDecl, Function, Procedure, Program};

use super::records::RuntimeType;
use super::values::key;
use super::{ControlFlow, Frame, RuntimeClass, RuntimeEnum};

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) types: HashMap<String, RuntimeType>,
    pub(crate) enums: HashMap<String, RuntimeEnum>,
    pub(crate) enum_members: HashMap<String, i64>,
    pub(crate) classes: HashMap<String, RuntimeClass>,
    pub(crate) procedures: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) declares: HashMap<String, DeclareDecl>,
    pub(crate) native_libraries: super::ffi::NativeLibraries,
    pub(crate) ffi_callbacks: HashMap<String, usize>,
    pub(crate) callback_trampolines: Vec<super::ffi::CallbackTrampoline>,
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
    pub(crate) class_modules: HashMap<String, Vec<String>>,
    pub(crate) type_modules: HashMap<String, Vec<String>>,
    pub(crate) enum_modules: HashMap<String, Vec<String>>,
    pub(crate) public_classes: HashSet<String>,
    pub(crate) public_types: HashSet<String>,
    pub(crate) public_enums: HashSet<String>,
    pub(crate) public_values: HashMap<String, Vec<String>>,
    pub(crate) rng: Pcg64,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            types: HashMap::new(),
            enums: HashMap::new(),
            enum_members: HashMap::new(),
            classes: HashMap::new(),
            procedures: HashMap::new(),
            functions: HashMap::new(),
            declares: HashMap::new(),
            native_libraries: super::ffi::NativeLibraries::default(),
            ffi_callbacks: HashMap::new(),
            callback_trampolines: Vec::new(),
            output: Vec::new(),
            option_base: 0,
            option_compare: crate::OptionCompare::Binary,
            call_stack: Vec::new(),
            scope_stack: Vec::new(),
            static_frames: HashMap::new(),
            err_number: 0,
            err_description: String::new(),
            err_source: String::new(),
            err_help_file: String::new(),
            err_help_context: 0,
            erl: 0,
            module_frames: HashMap::new(),
            module_imports: HashMap::new(),
            function_modules: HashMap::new(),
            sub_modules: HashMap::new(),
            class_modules: HashMap::new(),
            type_modules: HashMap::new(),
            enum_modules: HashMap::new(),
            public_classes: HashSet::new(),
            public_types: HashSet::new(),
            public_enums: HashSet::new(),
            public_values: HashMap::new(),
            rng: Pcg64::from_entropy(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeImport {
    pub(crate) qualifier: String,
    pub(crate) module: String,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            option_compare: crate::OptionCompare::Binary,
            rng: Pcg64::from_entropy(),
            ..Self::default()
        };
        interpreter.add_builtin_classes();
        interpreter.add_vba_constants();
        interpreter
    }

    fn add_vba_constants(&mut self) {
        let constants = [
            ("vbBinaryCompare", 0),
            ("vbTextCompare", 1),
            // VarType
            ("vbEmpty", 0),
            ("vbNull", 1),
            ("vbInteger", 2),
            ("vbLong", 3),
            ("vbSingle", 4),
            ("vbDouble", 5),
            ("vbCurrency", 6),
            ("vbDate", 7),
            ("vbString", 8),
            ("vbObject", 9),
            ("vbError", 10),
            ("vbBoolean", 11),
            ("vbVariant", 12),
            ("vbDataObject", 13),
            ("vbDecimal", 14),
            ("vbByte", 17),
            ("vbLongLong", 20),
            ("vbUserDefinedType", 36),
            ("vbArray", 8192),
            // CallByName
            ("VbMethod", 1),
            ("VbGet", 2),
            ("VbLet", 4),
            ("VbSet", 8),
        ];

        for (name, value) in constants {
            self.enum_members.insert(key(name), value);
        }
    }

    fn add_builtin_classes(&mut self) {
        self.classes.insert(
            key("Error"),
            RuntimeClass {
                name: "Error".to_string(),
                fields: vec![
                    crate::interpreter::records::RuntimeField {
                        name: "Number".to_string(),
                        ty: crate::runtime::TypeName::Integer,
                        array: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "Message".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "Description".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "Source".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "HelpFile".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "HelpContext".to_string(),
                        ty: crate::runtime::TypeName::Integer,
                        array: None,
                        initializer: None,
                        with_events: false,
                    },
                ],
                constants: Vec::new(),
                events: HashMap::new(),
                subs: HashMap::new(),
                functions: HashMap::new(),
                iterator: None,
                properties: HashMap::new(),
                enumerator_member: None,
                default_member: None,
            },
        );
    }

    pub(crate) fn clear_err(&mut self) {
        self.err_number = 0;
        self.err_description = String::new();
        self.err_source = String::new();
        self.err_help_file = String::new();
        self.err_help_context = 0;
    }

    pub(crate) fn set_err(&mut self, error: &Diagnostic, erl: i64) {
        if let Some(info) = &error.runtime_error {
            self.err_number = info.number;
            self.err_description = info.description.clone();
            self.err_source = info.source.clone();
            self.err_help_file = info.help_file.clone();
            self.err_help_context = info.help_context;
        } else {
            self.err_number = 1;
            self.err_description = error.message.to_string();
            self.err_source = "Valo.Runtime".to_string();
            self.err_help_file = String::new();
            self.err_help_context = 0;
        }
        self.erl = erl;
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
        self.register_declares(&program.declares, None);

        let mut frame = Frame::default();
        for var in &program.module_vars {
            let value = if let Some(initializer) = &var.initializer {
                Some(self.eval_expr(initializer, &mut frame)?)
            } else {
                None
            };
            let ty = var.ty.clone().unwrap_or_else(|| {
                value
                    .as_ref()
                    .map(Value::type_name)
                    .unwrap_or(TypeName::Variant)
            });
            frame.declare_module(
                &var.name,
                ty,
                var.array.clone(),
                self.option_base,
                false,
                value,
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
        self.terminate_frame_variables(frame, main.span)?;

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

    pub fn run_repl_snippet(
        &mut self,
        program: &Program,
        frame: &mut Frame,
    ) -> Result<Vec<String>, Diagnostic> {
        self.output.clear();
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
            if procedure.name.eq_ignore_ascii_case("main") {
                continue; // Main is run directly
            }
            self.procedures
                .insert(key(&procedure.name), procedure.clone());
        }
        for function in &program.functions {
            self.functions.insert(key(&function.name), function.clone());
        }
        self.register_declares(&program.declares, None);

        for var in &program.module_vars {
            if !frame.has_variable(&var.name) {
                let value = if let Some(initializer) = &var.initializer {
                    Some(self.eval_expr(initializer, frame)?)
                } else {
                    None
                };
                let ty = var.ty.clone().unwrap_or_else(|| {
                    value
                        .as_ref()
                        .map(Value::type_name)
                        .unwrap_or(TypeName::Variant)
                });
                frame.declare_module(
                    &var.name,
                    ty,
                    var.array.clone(),
                    self.option_base,
                    false,
                    value,
                    var.span,
                    &self.types,
                    &self.enums,
                )?;
            }
        }
        for const_decl in &program.module_consts {
            if !frame.has_variable(&const_decl.name) {
                let value = self.eval_expr(&const_decl.value, frame)?;
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
        }

        let Some(main) = program
            .procedures
            .iter()
            .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
        else {
            return Ok(self.output.clone());
        };

        // Execute statements from main block manually to keep frame persistent
        self.exec_block(&main.body, frame)?;
        Ok(self.output.clone())
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

        self.terminate_frame_variables(frame, main.span)?;
        let module_keys: Vec<_> = self.module_frames.keys().cloned().collect();
        for key in module_keys {
            if let Some(module_frame) = self.module_frames.remove(&key) {
                self.terminate_frame_variables(module_frame, main.span)?;
            }
        }

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
        for module in &project.modules {
            let module_key = super::values::key(&module.name);
            self.module_imports.insert(
                module_key,
                module
                    .imports
                    .iter()
                    .map(|import| RuntimeImport {
                        qualifier: import.qualifier.clone(),
                        module: super::values::key(&project.modules[import.module].name),
                    })
                    .collect(),
            );
        }
        let entry_key = super::values::key(&project.modules[project.entry].name);
        for module in &project.modules {
            self.option_base = module.program.option_base;
            self.option_compare = module.program.option_compare;
            let module_key = super::values::key(&module.name);
            for type_decl in &module.program.types {
                let qualified = qualified_symbol_key(&module_key, &type_decl.name);
                let mut runtime_type = RuntimeType::from(type_decl);
                runtime_type.name = qualified_display_name(&module.name, &type_decl.name);
                self.types.insert(qualified.clone(), runtime_type);
                if module_key == entry_key {
                    self.types.insert(
                        super::values::key(&type_decl.name),
                        RuntimeType::from(type_decl),
                    );
                }
                if module_key == entry_key || crate::modules::is_public(type_decl.visibility) {
                    self.type_modules
                        .entry(super::values::key(&type_decl.name))
                        .or_default()
                        .push(module_key.clone());
                    self.public_types.insert(qualified);
                }
            }
            for enum_decl in &module.program.enums {
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
                let qualified = qualified_symbol_key(&module_key, &enum_decl.name);
                self.enums.insert(
                    qualified.clone(),
                    RuntimeEnum {
                        name: qualified_display_name(&module.name, &enum_decl.name),
                        members,
                    },
                );
                if module_key == entry_key {
                    let enum_ = self.enums.get(&qualified).expect("inserted").clone();
                    self.enums
                        .insert(super::values::key(&enum_decl.name), enum_);
                }
                if module_key == entry_key || crate::modules::is_public(enum_decl.visibility) {
                    self.enum_modules
                        .entry(super::values::key(&enum_decl.name))
                        .or_default()
                        .push(module_key.clone());
                    self.public_enums.insert(qualified);
                }
            }
            for class_decl in &module.program.classes {
                let qualified = qualified_symbol_key(&module_key, &class_decl.name);
                let mut runtime_class = RuntimeClass::from(class_decl);
                runtime_class.name = qualified_display_name(&module.name, &class_decl.name);
                self.classes.insert(qualified.clone(), runtime_class);
                if module_key == entry_key {
                    self.classes.insert(
                        super::values::key(&class_decl.name),
                        RuntimeClass::from(class_decl),
                    );
                }
                if module_key == entry_key || crate::modules::is_public(class_decl.visibility) {
                    self.class_modules
                        .entry(super::values::key(&class_decl.name))
                        .or_default()
                        .push(module_key.clone());
                    self.public_classes.insert(qualified);
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
            self.register_declares(&module.program.declares, Some(&module_key));
        }
        for module in &project.modules {
            let module_key = super::values::key(&module.name);
            let mut frame = Frame::default();
            frame.set_module_key(module_key.clone());
            let mut public_values = Vec::new();
            for var in &module.program.module_vars {
                if crate::modules::is_public(var.visibility) {
                    public_values.push(super::values::key(&var.name));
                }
                let value = if let Some(initializer) = &var.initializer {
                    Some(self.eval_expr(initializer, &mut frame)?)
                } else {
                    None
                };
                let ty = var.ty.clone().unwrap_or_else(|| {
                    value
                        .as_ref()
                        .map(Value::type_name)
                        .unwrap_or(TypeName::Variant)
                });
                let ty = self.resolve_type_name(&ty, &frame, var.span)?;
                frame.declare_module(
                    &var.name,
                    ty,
                    var.array.clone(),
                    module.program.option_base,
                    false,
                    value,
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
                let ty = self.resolve_type_name(&ty, &frame, const_decl.span)?;
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

pub(crate) fn qualified_symbol_key(module_key: &str, name: &str) -> String {
    format!("{}.{}", module_key, super::values::key(name))
}

fn qualified_display_name(module_name: &str, name: &str) -> String {
    format!("{module_name}.{name}")
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

    pub(crate) fn resolve_type_name(
        &self,
        ty: &crate::runtime::TypeName,
        frame: &Frame,
        span: crate::runtime::Span,
    ) -> Result<crate::runtime::TypeName, Diagnostic> {
        let crate::runtime::TypeName::User(name) = ty else {
            return Ok(ty.clone());
        };
        let resolved = self.resolve_user_type_name(name, frame, span)?;
        Ok(crate::runtime::TypeName::User(resolved))
    }

    pub(crate) fn resolve_user_type_name(
        &self,
        name: &str,
        frame: &Frame,
        span: crate::runtime::Span,
    ) -> Result<String, Diagnostic> {
        if let Some((qualifier, member)) = name.split_once('.') {
            let module_key = self.resolve_module_qualifier(qualifier, frame, span)?;
            let qualified = qualified_symbol_key(&module_key, member);
            if self.classes.contains_key(&qualified) {
                self.ensure_public_qualified_type(&qualified, frame, &module_key, name, span)?;
                return Ok(qualified);
            }
            if self.types.contains_key(&qualified) {
                self.ensure_public_qualified_type(&qualified, frame, &module_key, name, span)?;
                return Ok(qualified);
            }
            if self.enums.contains_key(&qualified) {
                self.ensure_public_qualified_type(&qualified, frame, &module_key, name, span)?;
                return Ok(qualified);
            }
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_QUALIFIED_SYMBOL,
                format!(
                    "Module '{}' has no type, class, or enum '{}'",
                    qualifier, member
                ),
                Some(span),
            ));
        }

        if let Some(current) = frame.module_key() {
            let current_key = qualified_symbol_key(current, name);
            if self.classes.contains_key(&current_key)
                || self.types.contains_key(&current_key)
                || self.enums.contains_key(&current_key)
            {
                return Ok(current_key);
            }
        }

        self.resolve_unqualified_type(name, frame, span)
    }

    fn resolve_unqualified_type(
        &self,
        name: &str,
        frame: &Frame,
        span: crate::runtime::Span,
    ) -> Result<String, Diagnostic> {
        let mut candidates = Vec::new();
        candidates.extend(self.imported_type_candidates(name, frame, &self.class_modules));
        candidates.extend(self.imported_type_candidates(name, frame, &self.type_modules));
        candidates.extend(self.imported_type_candidates(name, frame, &self.enum_modules));
        candidates.sort();
        candidates.dedup();
        match candidates.len() {
            0 => Ok(super::values::key(name)),
            1 => Ok(qualified_symbol_key(&candidates[0], name)),
            _ => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::AMBIGUOUS_IMPORT,
                format!(
                    "Type '{}' is imported from multiple modules; use a module qualifier",
                    name
                ),
                Some(span),
            )),
        }
    }

    fn imported_type_candidates(
        &self,
        name: &str,
        frame: &Frame,
        modules_by_name: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let Some(modules) = modules_by_name.get(&super::values::key(name)) else {
            return Vec::new();
        };
        let Some(current) = frame.module_key() else {
            return modules.clone();
        };
        modules
            .iter()
            .filter(|module| {
                self.module_imports
                    .get(current)
                    .is_some_and(|imports| imports.iter().any(|import| &import.module == *module))
            })
            .cloned()
            .collect()
    }

    fn ensure_public_qualified_type(
        &self,
        qualified: &str,
        frame: &Frame,
        module_key: &str,
        name: &str,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        if frame.module_key() == Some(module_key)
            || self.public_classes.contains(qualified)
            || self.public_types.contains(qualified)
            || self.public_enums.contains(qualified)
        {
            Ok(())
        } else {
            Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PRIVATE_ACCESS,
                format!("Imported type '{}' is Private", name),
                Some(span),
            ))
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
                    crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                    format!("Enum member '{}' is not defined", name),
                    Some(expr.span),
                )
            }),
            ExprKind::Unary {
                op: UnaryOp::Negate,
                expr,
            } => Ok(-self.eval_enum_const_expr(expr, members)?),
            ExprKind::AddressOf(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "AddressOf is not allowed in constant expressions",
                Some(expr.span),
            )),
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

    pub(crate) fn maybe_terminate(
        &mut self,
        value: Value,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        match value {
            Value::Object(rc) if Rc::strong_count(&rc) == 1 => {
                let mut borrow = rc.borrow_mut();
                if borrow.terminated {
                    return Ok(());
                }
                borrow.terminated = true;
                let class_name = borrow.class_name.clone();
                drop(borrow);

                if let Some(class) = self.classes.get(&key(&class_name)).cloned()
                    && let Some(terminate) = class
                        .subs
                        .get("terminate")
                        .or_else(|| class.subs.get("class_terminate"))
                {
                    let mut frame = Frame::default();
                    frame.set_module_key(key(class_name.split('.').next().unwrap_or(&class_name)));
                    self.call_method_sub_values(
                        Value::Object(rc),
                        &terminate.name,
                        &[],
                        &mut frame,
                        span,
                    )?;
                }
            }
            Value::Array { elements, .. } => {
                for element in elements {
                    self.maybe_terminate(element, span)?;
                }
            }
            Value::Record { fields, .. } => {
                for field_value in fields.into_values() {
                    self.maybe_terminate(field_value, span)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn terminate_frame_variables(
        &mut self,
        frame: Frame,
        span: crate::runtime::Span,
    ) -> Result<(), Diagnostic> {
        for (_, variable) in frame.into_variables() {
            let value = variable.cell.borrow().clone();
            drop(variable);
            self.maybe_terminate(value, span)?;
        }
        Ok(())
    }
}
