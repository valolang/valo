use rand::SeedableRng;
use rand_pcg::Pcg64;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::runtime::{Diagnostic, TypeName, Value};
use crate::{ClassDecl, DeclareDecl, Function, Procedure, Program};

use super::records::{RuntimeInterface, RuntimeType};
use super::values::key;
use super::{ControlFlow, Frame, RuntimeClass, RuntimeEnum};

#[derive(Debug)]
pub struct Interpreter {
    pub(crate) types: HashMap<String, RuntimeType>,
    pub(crate) interfaces: HashMap<String, RuntimeInterface>,
    pub(crate) enums: HashMap<String, RuntimeEnum>,
    pub(crate) enum_members: HashMap<String, i64>,
    pub(crate) classes: HashMap<String, RuntimeClass>,
    pub(crate) shared_class_fields: HashMap<String, HashMap<String, Value>>,
    pub(crate) procedures: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) declares: HashMap<String, DeclareDecl>,
    pub(crate) native_libraries: super::ffi::NativeLibraries,
    pub(crate) native_cifs: HashMap<String, Rc<libffi::middle::Cif>>,
    pub(crate) ffi_callbacks: HashMap<String, usize>,
    pub(crate) callback_trampolines: Vec<super::ffi::CallbackTrampoline>,
    pub(crate) temporary_strings: Vec<String>,
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
    pub(crate) extension_methods: HashMap<String, Vec<String>>, // Type name -> Qualified method names
    pub(crate) class_modules: HashMap<String, Vec<String>>,
    pub(crate) merged_partial_class_modules: HashMap<String, HashSet<String>>,
    pub(crate) type_modules: HashMap<String, Vec<String>>,
    pub(crate) enum_modules: HashMap<String, Vec<String>>,
    pub(crate) interface_modules: HashMap<String, Vec<String>>,
    pub(crate) public_classes: HashSet<String>,
    pub(crate) public_types: HashSet<String>,
    pub(crate) public_enums: HashSet<String>,
    pub(crate) public_interfaces: HashSet<String>,
    pub(crate) public_values: HashMap<String, HashSet<String>>,
    pub(crate) rng: Pcg64,
    pub(crate) file_io: super::file_io::FileIoState,
    pub(crate) terminated: bool,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            types: HashMap::new(),
            interfaces: HashMap::new(),
            enums: HashMap::new(),
            enum_members: HashMap::new(),
            classes: HashMap::new(),
            shared_class_fields: HashMap::new(),
            procedures: HashMap::new(),
            functions: HashMap::new(),
            declares: HashMap::new(),
            native_libraries: super::ffi::NativeLibraries::default(),
            native_cifs: HashMap::new(),
            ffi_callbacks: HashMap::new(),
            callback_trampolines: Vec::new(),
            temporary_strings: Vec::new(),
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
            extension_methods: HashMap::new(),
            class_modules: HashMap::new(),
            merged_partial_class_modules: HashMap::new(),
            type_modules: HashMap::new(),
            enum_modules: HashMap::new(),
            interface_modules: HashMap::new(),
            public_classes: HashSet::new(),
            public_types: HashSet::new(),
            public_enums: HashSet::new(),
            public_interfaces: HashSet::new(),
            public_values: HashMap::new(),
            rng: Pcg64::from_entropy(),
            file_io: super::file_io::FileIoState::default(),
            terminated: false,
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
        for constant in crate::runtime::vba::VBA_CONSTANTS {
            if let crate::runtime::vba::VbaConstantValue::Integer(value) = constant.value {
                self.enum_members.insert(key(constant.name), value);
            }
        }
    }

    fn add_builtin_classes(&mut self) {
        self.classes.insert(
            key("Error"),
            RuntimeClass {
                name: "Error".to_string(),
                type_params: Vec::new(),
                inheritance: crate::ClassInheritance::Normal,
                base_class: None,
                fields: vec![
                    crate::interpreter::records::RuntimeField {
                        name: "Number".to_string(),
                        ty: crate::runtime::TypeName::Integer,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        collection_initializer: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "Message".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        collection_initializer: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "Description".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        collection_initializer: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "Source".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        collection_initializer: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "HelpFile".to_string(),
                        ty: crate::runtime::TypeName::String,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        collection_initializer: None,
                        initializer: None,
                        with_events: false,
                    },
                    crate::interpreter::records::RuntimeField {
                        name: "HelpContext".to_string(),
                        ty: crate::runtime::TypeName::Integer,
                        array: None,
                        as_new: false,
                        new_args: Vec::new(),
                        collection_initializer: None,
                        initializer: None,
                        with_events: false,
                    },
                ],
                shared_fields: Vec::new(),
                constants: Vec::new(),
                events: HashMap::new(),
                subs: HashMap::new(),
                shared_subs: HashMap::new(),
                functions: HashMap::new(),
                shared_functions: HashMap::new(),
                iterator: None,
                properties: HashMap::new(),
                operators: HashMap::new(),
                enumerator_member: None,
                default_member: None,
            },
        );
        self.classes.insert(
            key("Collection"),
            RuntimeClass {
                name: "Collection".to_string(),
                type_params: Vec::new(),
                inheritance: crate::ClassInheritance::Normal,
                base_class: None,
                fields: Vec::new(),
                shared_fields: Vec::new(),
                constants: Vec::new(),
                events: HashMap::new(),
                subs: HashMap::new(),
                shared_subs: HashMap::new(),
                functions: HashMap::new(),
                shared_functions: HashMap::new(),
                iterator: None,
                properties: HashMap::new(),
                operators: HashMap::new(),
                enumerator_member: None,
                default_member: Some("Item".to_string()),
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
        for interface_decl in &program.interfaces {
            self.interfaces.insert(
                key(&interface_decl.name),
                RuntimeInterface::from(interface_decl),
            );
        }

        let mut module_const_values = HashMap::new();
        let mut pending_consts: Vec<_> = program.module_consts.iter().collect();
        let mut progress = true;
        while progress && !pending_consts.is_empty() {
            progress = false;
            let mut next_pending = Vec::new();
            for const_decl in pending_consts {
                if let Ok(value) = self.eval_enum_const_expr(
                    &const_decl.value,
                    &HashMap::new(),
                    &module_const_values,
                ) {
                    module_const_values.insert(key(&const_decl.name), value);
                    progress = true;
                } else {
                    next_pending.push(const_decl);
                }
            }
            pending_consts = next_pending;
        }

        for enum_decl in &program.enums {
            let mut members = HashMap::new();
            let mut previous = -1;
            for member in &enum_decl.members {
                let value = if let Some(expr) = &member.value {
                    self.eval_enum_const_expr(expr, &members, &module_const_values)?
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
        self.apply_class_inheritance(crate::runtime::Span::empty(
            crate::runtime::FileId::default(),
        ))?;
        self.initialize_shared_class_fields(crate::runtime::Span::empty(
            crate::runtime::FileId::default(),
        ))?;
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
                &self,
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
                &self,
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
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(self.output.clone()),
            ControlFlow::Terminate => Ok(self.output.clone()),
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
            ControlFlow::ExitProperty
            | ControlFlow::ExitFor
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
        for interface_decl in &program.interfaces {
            self.interfaces.insert(
                key(&interface_decl.name),
                RuntimeInterface::from(interface_decl),
            );
        }

        let mut module_const_values = HashMap::new();
        let mut pending_consts: Vec<_> = program.module_consts.iter().collect();
        let mut progress = true;
        while progress && !pending_consts.is_empty() {
            progress = false;
            let mut next_pending = Vec::new();
            for const_decl in pending_consts {
                if let Ok(value) = self.eval_enum_const_expr(
                    &const_decl.value,
                    &HashMap::new(),
                    &module_const_values,
                ) {
                    module_const_values.insert(key(&const_decl.name), value);
                    progress = true;
                } else {
                    next_pending.push(const_decl);
                }
            }
            pending_consts = next_pending;
        }

        for enum_decl in &program.enums {
            let mut members = HashMap::new();
            let mut previous = -1;
            for member in &enum_decl.members {
                let value = if let Some(expr) = &member.value {
                    self.eval_enum_const_expr(expr, &members, &module_const_values)?
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
        self.initialize_shared_class_fields(crate::runtime::Span::empty(
            crate::runtime::FileId::default(),
        ))?;
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
                    self,
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
                    self,
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
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(self.output.clone()),
            ControlFlow::Terminate => Ok(self.output.clone()),
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
            ControlFlow::ExitProperty
            | ControlFlow::ExitFor
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
        let mut partial_class_groups: HashMap<String, Vec<(String, ClassDecl)>> = HashMap::new();
        for module in &project.modules {
            self.option_base = module.program.option_base;
            self.option_compare = module.program.option_compare;
            let module_key = super::values::key(&module.name);

            let mut module_const_values = HashMap::new();
            let mut pending_consts: Vec<_> = module.program.module_consts.iter().collect();
            let mut progress = true;
            while progress && !pending_consts.is_empty() {
                progress = false;
                let mut next_pending = Vec::new();
                for const_decl in pending_consts {
                    if let Ok(value) = self.eval_enum_const_expr(
                        &const_decl.value,
                        &HashMap::new(),
                        &module_const_values,
                    ) {
                        module_const_values.insert(super::values::key(&const_decl.name), value);
                        progress = true;
                    } else {
                        next_pending.push(const_decl);
                    }
                }
                pending_consts = next_pending;
            }

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
            for interface_decl in &module.program.interfaces {
                let qualified = qualified_symbol_key(&module_key, &interface_decl.name);
                let mut runtime_interface = RuntimeInterface::from(interface_decl);
                runtime_interface.name = qualified_display_name(&module.name, &interface_decl.name);
                self.interfaces.insert(qualified.clone(), runtime_interface);
                if module_key == entry_key {
                    self.interfaces.insert(
                        super::values::key(&interface_decl.name),
                        RuntimeInterface::from(interface_decl),
                    );
                }
                if module_key == entry_key || crate::modules::is_public(interface_decl.visibility) {
                    self.interface_modules
                        .entry(super::values::key(&interface_decl.name))
                        .or_default()
                        .push(module_key.clone());
                    self.public_interfaces.insert(qualified);
                }
            }
            for enum_decl in &module.program.enums {
                let mut members = HashMap::new();
                let mut previous = -1;
                for member in &enum_decl.members {
                    let value = if let Some(expr) = &member.value {
                        self.eval_enum_const_expr(expr, &members, &module_const_values)?
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
                if class_decl.is_partial
                    && (module_key == entry_key || crate::modules::is_public(class_decl.visibility))
                {
                    partial_class_groups
                        .entry(super::values::key(&class_decl.name))
                        .or_default()
                        .push((module_key.clone(), class_decl.clone()));
                }
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
            self.apply_class_inheritance(crate::runtime::Span::empty(
                crate::runtime::FileId::default(),
            ))?;
            for procedure in &module.program.procedures {
                let qualified = format!("{}::{}", module_key, super::values::key(&procedure.name));
                self.procedures.insert(qualified.clone(), procedure.clone());
                if module_key == entry_key || crate::modules::is_public(procedure.visibility) {
                    self.sub_modules
                        .entry(super::values::key(&procedure.name))
                        .or_default()
                        .push(module_key.clone());
                }
                if procedure
                    .attributes
                    .iter()
                    .any(|a| a.name.eq_ignore_ascii_case("Extension"))
                    && let Some(first_param) = procedure.params.first()
                {
                    let type_key = first_param.ty.display_name().to_lowercase();
                    self.extension_methods
                        .entry(type_key)
                        .or_default()
                        .push(qualified);
                }
            }
            for function in &module.program.functions {
                let qualified = format!("{}::{}", module_key, super::values::key(&function.name));
                self.functions.insert(qualified.clone(), function.clone());
                if module_key == entry_key || crate::modules::is_public(function.visibility) {
                    self.function_modules
                        .entry(super::values::key(&function.name))
                        .or_default()
                        .push(module_key.clone());
                }
                if function
                    .attributes
                    .iter()
                    .any(|a| a.name.eq_ignore_ascii_case("Extension"))
                    && let Some(first_param) = function.params.first()
                {
                    let type_key = first_param.ty.display_name().to_lowercase();
                    self.extension_methods
                        .entry(type_key)
                        .or_default()
                        .push(qualified);
                }
            }
            self.register_declares(&module.program.declares, Some(&module_key));
        }
        self.register_merged_partial_classes(partial_class_groups);
        self.apply_class_inheritance(crate::runtime::Span::empty(
            crate::runtime::FileId::default(),
        ))?;
        self.initialize_shared_class_fields(crate::runtime::Span::empty(
            crate::runtime::FileId::default(),
        ))?;
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
                    self,
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
                    self,
                )?;
            }
            self.public_values
                .insert(module_key.clone(), public_values.into_iter().collect());
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

fn merge_partial_runtime_class(target: &mut RuntimeClass, mut source: RuntimeClass) {
    target.fields.append(&mut source.fields);
    target.shared_fields.append(&mut source.shared_fields);
    target.constants.append(&mut source.constants);
    target.events.extend(source.events);
    target.subs.extend(source.subs);
    target.shared_subs.extend(source.shared_subs);
    target.functions.extend(source.functions);
    target.shared_functions.extend(source.shared_functions);
    target.properties.extend(source.properties);
    target.operators.extend(source.operators);

    if target.base_class.is_none() {
        target.base_class = source.base_class;
    }
    if target.iterator.is_none() {
        target.iterator = source.iterator;
    }
    if target.enumerator_member.is_none() {
        target.enumerator_member = source.enumerator_member;
    }
    if target.default_member.is_none() {
        target.default_member = source.default_member;
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
            let mut trace = String::from("Stack trace:");
            for frame in self.call_stack.iter().rev() {
                trace.push_str("\n  at ");
                trace.push_str(frame);
            }
            diagnostic
                .with_note(format!("while executing {}", self.call_stack.join(" -> ")))
                .with_note(trace)
        }
    }

    pub(crate) fn resolve_type_name(
        &self,
        ty: &crate::runtime::TypeName,
        frame: &Frame,
        span: crate::runtime::Span,
    ) -> Result<crate::runtime::TypeName, Diagnostic> {
        let crate::runtime::TypeName::User(name) = ty else {
            return match ty {
                crate::runtime::TypeName::GenericInstance { name, args } => {
                    let resolved = self.resolve_user_type_name(name, frame, span)?;
                    let args = args
                        .iter()
                        .map(|arg| self.resolve_type_name(arg, frame, span))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(crate::runtime::TypeName::GenericInstance {
                        name: resolved,
                        args,
                    })
                }
                crate::runtime::TypeName::Array(inner) => Ok(crate::runtime::TypeName::Array(
                    Box::new(self.resolve_type_name(inner, frame, span)?),
                )),
                _ => Ok(ty.clone()),
            };
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
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut parens = 0;
        for c in name.chars() {
            match c {
                '(' => {
                    parens += 1;
                    current.push(c);
                }
                ')' => {
                    parens -= 1;
                    current.push(c);
                }
                '.' if parens == 0 => {
                    parts.push(current.clone());
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        parts.push(current);

        let mut resolved = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                if parts.len() > 1 && self.resolve_module_qualifier(part, frame, span).is_ok() {
                    resolved = self.resolve_module_qualifier(part, frame, span).unwrap();
                } else if self.classes.contains_key(&key(part))
                    || self.types.contains_key(&key(part))
                    || self.interfaces.contains_key(&key(part))
                    || self.enums.contains_key(&key(part))
                {
                    resolved = key(part);
                    // Ensure the unqualified type is public if it comes from another module
                    let _ = self.resolve_unqualified_type(part, frame, span)?;
                } else if let Ok(full_resolution) = self.resolve_unqualified_type(name, frame, span)
                {
                    return Ok(full_resolution);
                } else {
                    return self.resolve_unqualified_type(part, frame, span);
                }
            } else {
                let module_key = resolved.clone();
                resolved = format!("{}.{}", resolved, key(part));
                if !self.classes.contains_key(&resolved)
                    && !self.types.contains_key(&resolved)
                    && !self.interfaces.contains_key(&resolved)
                    && !self.enums.contains_key(&resolved)
                {
                    return Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Symbol '{}' not found", resolved),
                        Some(span),
                    ));
                }
                if parts.len() > 1 && i == 1 && self.module_frames.contains_key(&module_key) {
                    // It's a module qualifier, ensure the type is public
                    self.ensure_public_qualified_type(&resolved, frame, &module_key, name, span)?;
                }
            }
        }
        Ok(resolved)
    }

    fn resolve_unqualified_type(
        &self,
        name: &str,
        frame: &Frame,
        span: crate::runtime::Span,
    ) -> Result<String, Diagnostic> {
        if let Some(current) = frame.module_key() {
            let qualified = qualified_symbol_key(current, name);
            if self.classes.contains_key(&qualified)
                || self.types.contains_key(&qualified)
                || self.interfaces.contains_key(&qualified)
                || self.enums.contains_key(&qualified)
            {
                return Ok(qualified);
            }
        }

        let mut candidates = Vec::new();
        candidates.extend(self.imported_type_candidates(name, frame, &self.class_modules));
        candidates.extend(self.imported_type_candidates(name, frame, &self.type_modules));
        candidates.extend(self.imported_type_candidates(name, frame, &self.enum_modules));
        candidates.extend(self.imported_type_candidates(name, frame, &self.interface_modules));
        candidates.sort();
        candidates.dedup();
        match candidates.len() {
            0 => {
                let key = super::values::key(name);
                Ok(key)
            }
            1 => {
                let module_key = &candidates[0];
                let qualified = qualified_symbol_key(module_key, name);
                self.ensure_public_qualified_type(&qualified, frame, module_key, name, span)?;
                Ok(qualified)
            }
            _ => {
                let name_key = super::values::key(name);
                if self
                    .merged_partial_class_modules
                    .get(&name_key)
                    .is_some_and(|modules| candidates.iter().all(|module| modules.contains(module)))
                {
                    Ok(name_key)
                } else {
                    Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::AMBIGUOUS_IMPORT,
                        format!(
                            "Type '{}' is imported from multiple modules; use a module qualifier",
                            name
                        ),
                        Some(span),
                    ))
                }
            }
        }
    }

    fn register_merged_partial_classes(
        &mut self,
        partial_class_groups: HashMap<String, Vec<(String, ClassDecl)>>,
    ) {
        for (class_key, partials) in partial_class_groups {
            if partials.len() < 2 {
                continue;
            }

            let mut partials = partials.into_iter();
            let Some((first_module, first)) = partials.next() else {
                continue;
            };
            let mut modules = HashSet::from([first_module]);
            let mut merged = RuntimeClass::from(&first);
            merged.name = first.name.clone();
            for (module, partial) in partials {
                modules.insert(module);
                merge_partial_runtime_class(&mut merged, RuntimeClass::from(&partial));
            }
            self.classes.insert(class_key.clone(), merged);
            self.merged_partial_class_modules
                .insert(class_key.clone(), modules);
            self.public_classes.insert(class_key);
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
                *module == current
                    || self.module_imports.get(current).is_some_and(|imports| {
                        imports.iter().any(|import| &import.module == *module)
                    })
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
            || self.public_interfaces.contains(qualified)
            || self.public_enums.contains(qualified)
        {
            Ok(())
        } else {
            Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE,
                format!("Imported type '{}' is Private", name),
                Some(span),
            ))
        }
    }

    fn eval_enum_const_expr(
        &self,
        expr: &crate::Expr,
        members: &HashMap<String, i64>,
        module_consts: &HashMap<String, i64>,
    ) -> Result<i64, Diagnostic> {
        use crate::{BinaryOp, ExprKind, UnaryOp};
        match &expr.kind {
            ExprKind::Integer(value) => Ok(*value),
            ExprKind::Long(value) => Ok(*value as i64),
            ExprKind::LongLong(value) => Ok(*value),
            ExprKind::Variable(name) => {
                let name_key = key(name);
                if let Some(&val) = members.get(&name_key) {
                    Ok(val)
                } else if let Some(&val) = module_consts.get(&name_key) {
                    Ok(val)
                } else {
                    Err(Diagnostic::new(
                        crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                        format!("Enum member '{}' is not defined", name),
                        Some(expr.span),
                    ))
                }
            }
            ExprKind::Unary {
                op: UnaryOp::Negate,
                expr,
            } => Ok(-self.eval_enum_const_expr(expr, members, module_consts)?),
            ExprKind::Unary {
                op: UnaryOp::Positive,
                expr,
            } => self.eval_enum_const_expr(expr, members, module_consts),
            ExprKind::AddressOf(_) => Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::TYPE_MISMATCH,
                "AddressOf is not allowed in constant expressions",
                Some(expr.span),
            )),
            ExprKind::PassingModeOverride { expr: inner, .. } => {
                self.eval_enum_const_expr(inner, members, module_consts)
            }
            ExprKind::Binary { left, op, right } => {
                let left = self.eval_enum_const_expr(left, members, module_consts)?;
                let right = self.eval_enum_const_expr(right, members, module_consts)?;
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
            Value::Array(array) => {
                if let Ok(array) = Rc::try_unwrap(array) {
                    for element in array.elements {
                        self.maybe_terminate(element, span)?;
                    }
                }
            }
            Value::Record(record) => {
                if let Ok(record) = Rc::try_unwrap(record) {
                    for field_value in record.fields.into_values() {
                        self.maybe_terminate(field_value, span)?;
                    }
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
            let value = variable.borrow().clone();
            drop(variable);
            self.maybe_terminate(value, span)?;
        }
        Ok(())
    }
}
