use std::collections::HashMap;

use crate::runtime::{Diagnostic, TypeName};
use crate::{
    ArrayDecl, AssignTarget, BinaryOp, CaseCompareOp, CaseItem, ClassMember, DoLoopCondition,
    ExitTarget, Expr, ExprKind, Function, OnErrorMode, Parameter, PassingMode, Procedure, Program,
    PropertyKind, ReDimTarget, ResumeTarget, Stmt, UnaryOp, Visibility,
};

use crate::frontend::semantics::context::Context;
use crate::frontend::semantics::symbols::{
    CallableSig, Options, Overloads, ParamSig, Signatures, VarType, key,
};
use crate::frontend::semantics::types::{
    ClassEventSig, ClassFieldSig, ClassMethodSig, ClassPropertySig, ClassSig, DelegateSig, EnumSig,
    FieldSig, InterfaceSig, MethodOverloads, PropertyAccessorSig, TypeRegistry, TypeSig,
};

#[path = "builtin_types.rs"]
mod builtin_types;
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
    collect_types_in_scope, ensure_const_expr, params_to_sigs, validate_function,
    validate_procedure,
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
            crate::runtime::DiagnosticCode::ENTRY_POINT,
            "Program must contain Sub Main()",
            None,
        ));
    }

    if let Some(main) = main
        && !main.params.is_empty()
    {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ENTRY_POINT,
            "Sub Main() cannot have parameters",
            Some(main.span),
        ));
    }

    validate_bodies(
        program,
        &types,
        &signatures,
        &module_symbols,
        program_options(program),
    )
}

pub fn validate_project(project: &crate::modules::Project) -> Result<(), Diagnostic> {
    validate_project_with_entry_requirement(project, true)
}

/// Validates a project the way `valo check` needs to.
///
/// This runs the same analysis as `validate_project` but does not insist on a
/// `Sub Main`, because a library or a single module under review is legitimately
/// missing one. It used to skip module validation entirely, which made `check`
/// report success on code that failed the moment it ran.
pub fn validate_project_for_check(project: &crate::modules::Project) -> Result<(), Diagnostic> {
    validate_project_with_entry_requirement(project, false)
}

fn validate_project_with_entry_requirement(
    project: &crate::modules::Project,
    require_entry_main: bool,
) -> Result<(), Diagnostic> {
    let _project_index = crate::frontend::semantics::hir::build_project_index(project)?;
    for (index, module) in project.modules.iter().enumerate() {
        let require_main = require_entry_main && index == project.entry;
        validate_module(&module.program, require_main, &module.imports, project)?;
        validate_import_aliases(module, project)?;
    }
    Ok(())
}

/// The directives a program declared, as one value.
pub(super) fn program_options(program: &Program) -> Options {
    Options {
        explicit: program.option_explicit,
        strict: program.option_strict,
    }
}

fn validate_module(
    program: &Program,
    require_main: bool,
    imports: &[crate::modules::ResolvedImport],
    project: &crate::modules::Project,
) -> Result<(), Diagnostic> {
    let types = module_type_scope(program, imports, project)?;
    let mut signatures = collect_signatures(program, &types)?;
    merge_imported_callables(imports, project, &mut signatures)?;
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
            crate::runtime::DiagnosticCode::ENTRY_POINT,
            "Program must contain Sub Main()",
            None,
        ));
    }
    if let Some(main) = main
        && !main.params.is_empty()
    {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::ENTRY_POINT,
            "Sub Main() cannot have parameters",
            Some(main.span),
        ));
    }

    validate_bodies(
        program,
        &types,
        &signatures,
        &module_symbols,
        program_options(program),
    )
}

/// The types a module can name: its own, plus those of the modules it imports.
///
/// Imported types are in scope while a module's own declarations are checked,
/// since a member declared `As Thing` may well name an import.
fn module_type_scope(
    program: &Program,
    imports: &[crate::modules::ResolvedImport],
    project: &crate::modules::Project,
) -> Result<TypeRegistry, Diagnostic> {
    let mut imported = TypeRegistry::default();
    merge_imported_types(imports, project, &mut imported)?;
    let mut types = collect_types_in_scope(program, &imported)?;
    merge_imported_types(imports, project, &mut types)?;
    merge_project_partial_classes(program, project, &mut types)?;
    Ok(types)
}

/// Brings the types and extension methods of imported modules into scope.
///
/// Each module is validated on its own, so without this an imported class or an
/// `<Extension()>` method declared elsewhere looks undefined. Locally declared
/// names win, since a module's own declarations shadow anything it imports.
fn merge_imported_types(
    imports: &[crate::modules::ResolvedImport],
    project: &crate::modules::Project,
    types: &mut TypeRegistry,
) -> Result<(), Diagnostic> {
    merge_imported_types_seen(
        imports,
        project,
        types,
        &mut std::collections::HashSet::new(),
    )
}

/// The same, refusing to walk in a circle.
///
/// Modules can import each other: a directory of `.bas` and `.cls` files is
/// loaded as one group where each sees all the others. Following an import's
/// own imports without noticing runs out of stack rather than out of modules.
///
/// `on_path` holds the modules currently being followed, not every module
/// already seen. The difference matters: one module can legitimately be
/// collected twice, once into the importer's scope and once into that of
/// another import that also names it.
fn merge_imported_types_seen(
    imports: &[crate::modules::ResolvedImport],
    project: &crate::modules::Project,
    types: &mut TypeRegistry,
    on_path: &mut std::collections::HashSet<usize>,
) -> Result<(), Diagnostic> {
    for import in imports {
        if on_path.contains(&import.module) {
            continue;
        }
        let Some(imported) = project.modules.get(import.module) else {
            continue;
        };
        // Against the imported module's own scope, not this one's. A signature
        // written `As Thing` in a module that imports `Thing` is well-formed
        // there, and stays well-formed when a third module imports it without
        // knowing `Thing` at all. `merge_imported_callables` does the same for
        // procedures; this is the type half of the same rule.
        let mut outer = TypeRegistry::default();
        on_path.insert(import.module);
        let followed = merge_imported_types_seen(&imported.imports, project, &mut outer, on_path);
        on_path.remove(&import.module);
        followed?;
        let imported_types = collect_types_in_scope(&imported.program, &outer)?;

        // Imported types are reachable both bare and through the import
        // qualifier, so register `PersonRecord` and `Models.PersonRecord`.
        let qualifier = key(&import.qualifier);
        let register = |bare: String, sig_name: &str| {
            let qualified = format!("{}.{}", qualifier, key(sig_name));
            (bare, qualified)
        };
        for (name, sig) in imported_types.types {
            let (bare, qualified) = register(name, &sig.name);
            types.types.entry(qualified).or_insert(sig.clone());
            types.types.entry(bare).or_insert(sig);
        }
        for (name, sig) in imported_types.enums {
            let (bare, qualified) = register(name, &sig.name);
            types.enums.entry(qualified).or_insert(sig.clone());
            types.enums.entry(bare).or_insert(sig);
        }
        for (name, sig) in imported_types.interfaces {
            let (bare, qualified) = register(name, &sig.name);
            types.interfaces.entry(qualified).or_insert(sig.clone());
            types.interfaces.entry(bare).or_insert(sig);
        }
        for (name, sig) in imported_types.classes {
            let (bare, qualified) = register(name, &sig.name);
            merge_class(types.classes.entry(qualified), sig.clone());
            merge_class(types.classes.entry(bare), sig);
        }
    }
    Ok(())
}

/// Brings the callables of imported modules into scope.
///
/// Split from the type half because signatures can only be collected once the
/// types they mention are known.
fn merge_imported_callables(
    imports: &[crate::modules::ResolvedImport],
    project: &crate::modules::Project,
    signatures: &mut Signatures,
) -> Result<(), Diagnostic> {
    for import in imports {
        let Some(imported) = project.modules.get(import.module) else {
            continue;
        };
        let qualifier = key(&import.qualifier);
        // Against the imported module's own scope, not this one's. A procedure
        // declared `As Thing` in a module that imports `Thing` is well-formed
        // there, and stays well-formed when a third module imports it without
        // knowing `Thing` at all.
        let imported_types = module_type_scope(&imported.program, &imported.imports, project)?;
        let imported_signatures = collect_signatures(&imported.program, &imported_types)?;

        for (type_key, methods) in imported_signatures.extension_methods {
            signatures
                .extension_methods
                .entry(type_key)
                .or_default()
                .extend(methods);
        }
        // A locally declared name shadows an import outright, overloads and
        // all: `or_insert` keeps whatever is already there rather than adding
        // the imported procedures to it.
        for (name, sigs) in imported_signatures.functions {
            let qualified = qualified_overload_key(&qualifier, &sigs, &name);
            signatures
                .functions
                .entry(qualified)
                .or_insert(sigs.clone());
            signatures.functions.entry(name).or_insert(sigs);
        }
        for (name, sigs) in imported_signatures.subs {
            let qualified = qualified_overload_key(&qualifier, &sigs, &name);
            signatures.subs.entry(qualified).or_insert(sigs.clone());
            signatures.subs.entry(name).or_insert(sigs);
        }
    }
    Ok(())
}

/// The key an imported procedure is reachable under when qualified.
///
/// Overloads share a name by definition, so any of them names the set.
fn qualified_overload_key(qualifier: &str, sigs: &Overloads, fallback: &str) -> String {
    let name = sigs.first().map_or(fallback, |sig| sig.name.as_str());
    format!("{}.{}", qualifier, key(name))
}

/// Completes this module's `Partial Class` declarations with the halves
/// declared in other modules.
///
/// A partial class is one class spread across files, so each half has to see
/// the members of the others even without an explicit import between them.
fn merge_project_partial_classes(
    program: &Program,
    project: &crate::modules::Project,
    types: &mut TypeRegistry,
) -> Result<(), Diagnostic> {
    let local_partials: Vec<String> = program
        .classes
        .iter()
        .filter(|class| class.is_partial)
        .map(|class| key(&class.name))
        .collect();
    if local_partials.is_empty() {
        return Ok(());
    }

    for module in &project.modules {
        let contributes = module
            .program
            .classes
            .iter()
            .any(|class| class.is_partial && local_partials.contains(&key(&class.name)));
        if !contributes {
            continue;
        }
        let other_types = collect_types(&module.program)?;
        for name in &local_partials {
            if let Some(sig) = other_types.classes.get(name) {
                merge_class(types.classes.entry(name.clone()), sig.clone());
            }
        }
    }
    Ok(())
}

/// Records an imported class, folding it into a same-named local class.
///
/// A `Partial Class` can be split across modules, so the halves have to be
/// merged rather than one shadowing the other. Members already present locally
/// win, which keeps a module's own declarations authoritative.
fn merge_class(entry: std::collections::hash_map::Entry<'_, String, ClassSig>, imported: ClassSig) {
    match entry {
        std::collections::hash_map::Entry::Vacant(slot) => {
            slot.insert(imported);
        }
        std::collections::hash_map::Entry::Occupied(mut slot) => {
            let existing = slot.get_mut();
            for (name, sig) in imported.fields {
                existing.fields.entry(name).or_insert(sig);
            }
            for (name, sig) in imported.subs {
                existing.subs.entry(name).or_insert(sig);
            }
            for (name, sig) in imported.functions {
                existing.functions.entry(name).or_insert(sig);
            }
            for (name, sig) in imported.properties {
                existing.properties.entry(name).or_insert(sig);
            }
            for (name, sig) in imported.events {
                existing.events.entry(name).or_insert(sig);
            }
            for (kind, sig) in imported.operators {
                existing.operators.entry(kind).or_insert(sig);
            }
            if existing.default_property.is_none() {
                existing.default_property = imported.default_property;
            }
            if existing.enumerator.is_none() {
                existing.enumerator = imported.enumerator;
            }
        }
    }
}

/// Validates every procedure, function, structure, and class body in a program.
///
/// Shared by the single-program and project entry points so both report the
/// same diagnostics; the project path used to stop before this, which left
/// `valo check` reporting success on code that failed as soon as it ran.
fn validate_bodies(
    program: &Program,
    types: &TypeRegistry,
    signatures: &Signatures,
    module_symbols: &HashMap<String, VarType>,
    options: Options,
) -> Result<(), Diagnostic> {
    // A declaration's constraints are in scope for its body and nowhere else,
    // so each body is checked against a registry that carries its own.
    for procedure in &program.procedures {
        let types = types.with_constraints(&procedure.generic_constraints);
        validate_procedure(procedure, &types, signatures, module_symbols, options)?;
    }
    for function in &program.functions {
        let types = types.with_constraints(&function.generic_constraints);
        validate_function(function, &types, signatures, module_symbols, options)?;
    }
    for type_decl in &program.types {
        if type_decl.kind == crate::TypeKind::Structure {
            validate_structure(type_decl, types, signatures, module_symbols, options)?;
        }
    }
    for class_decl in &program.classes {
        validate_class(class_decl, types, signatures, module_symbols, options)?;
    }
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
