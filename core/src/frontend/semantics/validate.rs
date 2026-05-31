use std::collections::HashMap;

use crate::runtime::{Diagnostic, TypeName};
use crate::{
    ArrayDecl, AssignTarget, BinaryOp, CaseCompareOp, CaseItem, ClassMember, DoLoopCondition,
    ExitTarget, Expr, ExprKind, Function, OnErrorMode, Parameter, PassingMode, Procedure, Program,
    PropertyKind, ReDimTarget, ResumeTarget, Stmt, UnaryOp, Visibility,
};

use crate::frontend::semantics::context::Context;
use crate::frontend::semantics::symbols::{CallableSig, ParamSig, Signatures, VarType, key};
use crate::frontend::semantics::types::{
    ClassEventSig, ClassFieldSig, ClassMethodSig, ClassPropertySig, ClassSig, EnumSig, FieldSig,
    InterfaceSig, PropertyAccessorSig, TypeRegistry, TypeSig,
};

#[path = "validate_classes.rs"]
mod validate_classes;
#[path = "validate_declarations.rs"]
mod validate_declarations;
#[path = "validate_expressions.rs"]
mod validate_expressions;
#[path = "validate_statements.rs"]
mod validate_statements;

use validate_classes::{validate_class, validate_structure};
use validate_declarations::{
    add_module_symbols, add_parameters, collect_module_symbols, collect_signatures, collect_types,
    ensure_const_expr, params_to_sigs, validate_function, validate_procedure,
};
use validate_expressions::*;
pub(super) use validate_statements::{LoopContext, StmtValidation, validate_statements};

pub fn validate(program: &Program) -> Result<(), Diagnostic> {
    validate_internal(program, true)
}

pub fn validate_snippet(program: &Program) -> Result<(), Diagnostic> {
    validate_internal(program, false)
}

fn validate_internal(program: &Program, require_main: bool) -> Result<(), Diagnostic> {
    let types = collect_types(program)?;
    let signatures = collect_signatures(program, &types)?;
    let mut module_symbols = collect_module_symbols(program, &types, &signatures)?;
    for import in &program.imports {
        let qualifier = import
            .alias
            .clone()
            .unwrap_or_else(|| import.module.clone());
        module_symbols.insert(
            key(&qualifier),
            VarType::Scalar(Visibility::Public, TypeName::Variant),
        );
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
        validate_procedure(
            procedure,
            &types,
            &signatures,
            &module_symbols,
            program.option_explicit,
        )?;
    }

    for function in &program.functions {
        validate_function(
            function,
            &types,
            &signatures,
            &module_symbols,
            program.option_explicit,
        )?;
    }
    for type_decl in &program.types {
        if type_decl.kind == crate::TypeKind::Structure {
            validate_structure(
                type_decl,
                &types,
                &signatures,
                &module_symbols,
                program.option_explicit,
            )?;
        }
    }
    for class_decl in &program.classes {
        validate_class(
            class_decl,
            &types,
            &signatures,
            &module_symbols,
            program.option_explicit,
        )?;
    }

    Ok(())
}

pub fn validate_project(project: &crate::modules::Project) -> Result<(), Diagnostic> {
    validate_project_with_entry_requirement(project, true)
}

pub fn validate_project_for_check(project: &crate::modules::Project) -> Result<(), Diagnostic> {
    let _project_index = crate::frontend::semantics::hir::build_project_index(project)?;
    for module in &project.modules {
        validate_import_aliases(module, project)?;
    }
    Ok(())
}

fn validate_project_with_entry_requirement(
    project: &crate::modules::Project,
    require_entry_main: bool,
) -> Result<(), Diagnostic> {
    let _project_index = crate::frontend::semantics::hir::build_project_index(project)?;
    for (index, module) in project.modules.iter().enumerate() {
        let require_main = require_entry_main && index == project.entry;
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
        module_symbols.insert(
            key(&import.qualifier),
            VarType::Scalar(Visibility::Public, TypeName::Variant),
        );
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

    // Project validation currently verifies declarations, import graph, and entry
    // shape. Body-level cross-module checking is intentionally left to runtime
    // resolution for the import MVP so the single-file validator remains intact.
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
