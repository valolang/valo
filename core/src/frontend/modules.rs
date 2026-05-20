//! Valo Module System
//!
//! Handles discovery, loading, and resolution of Valo modules and projects.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::frontend::ast::{ImportDecl, Program};
use crate::frontend::parser::parse_source;
use crate::runtime::{Diagnostic, DiagnosticCode, Span};
use crate::Visibility;

#[derive(Debug, Clone)]
pub struct Project {
    pub entry: usize,
    pub modules: Vec<LoadedModule>,
}

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub name: String,
    pub path: PathBuf,
    pub program: Program,
    pub imports: Vec<ResolvedImport>,
}

#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub module: usize,
    pub requested: String,
    pub qualifier: String,
    pub span: Span,
}

pub fn load_project(entry_path: impl AsRef<Path>) -> Result<Project, Diagnostic> {
    let mut loader = ModuleLoader::default();
    let entry = loader.load(entry_path.as_ref(), &mut Vec::new())?;
    Ok(Project {
        entry,
        modules: loader.modules,
    })
}

#[derive(Default)]
struct ModuleLoader {
    modules: Vec<LoadedModule>,
    by_path: HashMap<PathBuf, usize>,
}

impl ModuleLoader {
    fn load(&mut self, path: &Path, stack: &mut Vec<PathBuf>) -> Result<usize, Diagnostic> {
        let canonical = fs::canonicalize(path).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::MODULE_NOT_FOUND,
                format!(
                    "Module '{}' could not be found near '{}': {err}",
                    path.display(),
                    path.display()
                ),
                None,
            )
        })?;

        if stack.iter().any(|entry| entry == &canonical) {
            return Err(Diagnostic::new(
                DiagnosticCode::IMPORT_CYCLE,
                format!("Import cycle detected at '{}'", canonical.display()),
                None,
            ));
        }
        if let Some(index) = self.by_path.get(&canonical).copied() {
            return Ok(index);
        }

        stack.push(canonical.clone());
        let source = fs::read_to_string(&canonical).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::MODULE_NOT_FOUND,
                format!("Module '{}' could not be read: {err}", canonical.display()),
                None,
            )
        })?;
        let program = parse_source(&source)?;
        let name = module_name(&canonical);
        let index = self.modules.len();
        self.by_path.insert(canonical.clone(), index);
        self.modules.push(LoadedModule {
            name,
            path: canonical.clone(),
            program,
            imports: Vec::new(),
        });

        let imports = self.modules[index].program.imports.clone();
        let mut aliases = HashSet::new();
        let mut resolved = Vec::new();
        for import in imports {
            let target = resolve_import_path(&canonical, &import)?;
            let target_index = self.load(&target, stack)?;
            let qualifier = import
                .alias
                .clone()
                .unwrap_or_else(|| import.module.clone());
            let qualifier_key = qualifier.to_ascii_lowercase();
            if !aliases.insert(qualifier_key) {
                return Err(Diagnostic::new(
                    DiagnosticCode::DUPLICATE_IMPORT,
                    format!("Import alias '{}' is already used", qualifier),
                    Some(import.span),
                )
                .with_primary_label("duplicate import alias"));
            }
            resolved.push(ResolvedImport {
                module: target_index,
                requested: import.module,
                qualifier,
                span: import.span,
            });
        }
        self.modules[index].imports = resolved;
        stack.pop();
        Ok(index)
    }
}

fn module_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_string()
}

fn resolve_import_path(current: &Path, import: &ImportDecl) -> Result<PathBuf, Diagnostic> {
    let dir = current.parent().unwrap_or_else(|| Path::new("."));
    let target_valo = format!("{}.valo", import.module).to_ascii_lowercase();
    let target_bas = format!("{}.bas", import.module).to_ascii_lowercase();
    let target_cls = format!("{}.cls", import.module).to_ascii_lowercase();
    let mut matches = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::MODULE_NOT_FOUND,
            format!(
                "Module '{}' could not be found near '{}': {err}",
                import.module,
                dir.display()
            ),
            Some(import.span),
        )
    })? {
        let entry = entry.map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::MODULE_NOT_FOUND,
                format!(
                    "Module '{}' could not be found near '{}': {err}",
                    import.module,
                    dir.display()
                ),
                Some(import.span),
            )
        })?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name == target_valo || name == target_bas || name == target_cls {
            matches.push(entry.path());
        }
    }
    match matches.len() {
        0 => Err(Diagnostic::new(
            DiagnosticCode::MODULE_NOT_FOUND,
            format!(
                "Module '{}' could not be found near '{}'.",
                import.module,
                dir.display()
            ),
            Some(import.span),
        )
        .with_primary_label("unresolved import")),
        1 => Ok(matches.remove(0)),
        _ => Err(Diagnostic::new(
            DiagnosticCode::CASE_COLLISION,
            format!(
                "Import '{}' has multiple case-only filename matches near '{}'",
                import.module,
                dir.display()
            ),
            Some(import.span),
        )
        .with_primary_label("case-colliding module import")),
    }
}

pub(crate) fn is_public(visibility: Visibility) -> bool {
    visibility == Visibility::Public
}
