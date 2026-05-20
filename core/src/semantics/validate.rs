use std::collections::HashMap;

use crate::runtime::{Diagnostic, TypeName};
use crate::{
    AssignTarget, BinaryOp, CaseCompareOp, CaseItem, ClassMember, DoLoopCondition, ExitTarget,
    Expr, ExprKind, Function, OnErrorMode, Parameter, PassingMode, Procedure, Program,
    PropertyKind, ResumeTarget, Stmt, UnaryOp, Visibility,
};

use crate::semantics::context::Context;
use crate::semantics::symbols::{CallableSig, ParamSig, Signatures, VarType, key};
use crate::semantics::types::{
    ClassEventSig, ClassFieldSig, ClassMethodSig, ClassPropertySig, ClassSig, EnumSig, FieldSig,
    PropertyAccessorSig, TypeRegistry, TypeSig,
};

#[path = "validate_classes.rs"]
mod validate_classes;
#[path = "validate_declarations.rs"]
mod validate_declarations;
#[path = "validate_expressions.rs"]
mod validate_expressions;
#[path = "validate_statements.rs"]
mod validate_statements;

use validate_classes::validate_class;
use validate_declarations::{
    add_module_symbols, add_parameters, collect_module_symbols, collect_signatures, collect_types,
    ensure_const_expr, validate_function, validate_procedure,
};
use validate_expressions::*;
use validate_statements::validate_statements;

#[derive(Debug, Clone, Copy, Default)]
struct LoopContext {
    for_depth: usize,
    while_depth: usize,
    do_depth: usize,
}

impl LoopContext {
    fn in_for(mut self) -> Self {
        self.for_depth += 1;
        self
    }

    fn in_while(mut self) -> Self {
        self.while_depth += 1;
        self
    }

    fn in_do(mut self) -> Self {
        self.do_depth += 1;
        self
    }
}

pub fn validate(program: &Program) -> Result<(), Diagnostic> {
    let types = collect_types(program)?;
    let signatures = collect_signatures(program, &types)?;
    let mut module_symbols = collect_module_symbols(program, &types, &signatures)?;
    for import in &program.imports {
        let qualifier = import.alias.clone().unwrap_or_else(|| import.module.clone());
        module_symbols.insert(key(&qualifier), VarType::Scalar(TypeName::Variant));
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

    if !main.params.is_empty() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "Sub Main() cannot have parameters",
            Some(main.span),
        ));
    }

    for procedure in &program.procedures {
        validate_procedure(procedure, &types, &signatures, &module_symbols)?;
    }

    for function in &program.functions {
        validate_function(function, &types, &signatures, &module_symbols)?;
    }
    for class_decl in &program.classes {
        validate_class(class_decl, &types, &signatures, &module_symbols)?;
    }

    Ok(())
}

pub fn validate_project(project: &crate::modules::Project) -> Result<(), Diagnostic> {
    for (index, module) in project.modules.iter().enumerate() {
        let require_main = index == project.entry;
        validate_module(&module.program, require_main, &module.imports)?;
        validate_import_aliases(module, project)?;
    }
    Ok(())
}

fn validate_module(
    program: &Program,
    require_main: bool,
    imports: &[crate::modules::ResolvedImport],
) -> Result<(), Diagnostic> {
    let types = collect_types(program)?;
    let signatures = collect_signatures(program, &types)?;
    let mut module_symbols = collect_module_symbols(program, &types, &signatures)?;
    for import in imports {
        module_symbols.insert(key(&import.qualifier), VarType::Scalar(TypeName::Variant));
    }

    let main = program
        .procedures
        .iter()
        .find(|procedure| procedure.name.eq_ignore_ascii_case("main"));
    if require_main && main.is_none() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "Program must contain Sub Main()",
            None,
        ));
    }
    if let Some(main) = main
        && !main.params.is_empty()
    {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::GENERIC,
            "Sub Main() cannot have parameters",
            Some(main.span),
        ));
    }

    for procedure in &program.procedures {
        validate_procedure(procedure, &types, &signatures, &module_symbols)?;
    }

    for function in &program.functions {
        validate_function(function, &types, &signatures, &module_symbols)?;
    }

    for class_decl in &program.classes {
        validate_class(class_decl, &types, &signatures, &module_symbols)?;
    }
    // Project validation currently verifies declarations, import graph, and entry
    // shape.
    Ok(())
}

fn validate_import_aliases(
    module: &crate::modules::LoadedModule,
    project: &crate::modules::Project,
) -> Result<(), Diagnostic> {
    let mut aliases = HashMap::new();
    for import in &module.imports {
        let alias_key = key(&import.qualifier);
        if aliases.insert(alias_key, import.span).is_some() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_IMPORT,
                format!("Import alias '{}' is already used", import.qualifier),
                Some(import.span),
            ));
        }
        let imported = &project.modules[import.module];
        if module
            .program
            .procedures
            .iter()
            .any(|decl| decl.name.eq_ignore_ascii_case(&import.qualifier))
            || module
                .program
                .functions
                .iter()
                .any(|decl| decl.name.eq_ignore_ascii_case(&import.qualifier))
            || module
                .program
                .classes
                .iter()
                .any(|decl| decl.name.eq_ignore_ascii_case(&import.qualifier))
        {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::DUPLICATE_IMPORT,
                format!(
                    "Import alias '{}' conflicts with a top-level declaration",
                    import.qualifier
                ),
                Some(import.span),
            ));
        }
        let _ = imported;
    }
    Ok(())
}
