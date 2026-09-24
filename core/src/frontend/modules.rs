//! Valo Module System
//!
//! Handles discovery, loading, and resolution of Valo modules and projects.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::Visibility;
use crate::frontend::ast::{ImportDecl, Program};
use crate::frontend::parser::parse_source_with_id;
use crate::runtime::{Diagnostic, DiagnosticCode, FileId, SourceMap, Span};

#[derive(Debug, Clone)]
pub struct Project {
    pub entry: usize,
    pub modules: Vec<LoadedModule>,
    pub source_map: SourceMap,
}

/// One loaded source set, with a deterministic native declaration view.
/// Project validation retains each file's import scope; the combined view is
/// constructed only after that validation, for the current whole-module native
/// lowering path. Every declaration keeps its original source span.
pub struct Compilation<'a> {
    pub project: &'a Project,
    pub program: Program,
}

impl<'a> Compilation<'a> {
    pub fn for_native(project: &'a Project) -> Result<Self, Diagnostic> {
        crate::frontend::semantics::validate_project_for_check(project)?;
        let mut program = project.modules[project.entry].program.clone();
        let mut others = project
            .modules
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != project.entry)
            .map(|(_, module)| module)
            .collect::<Vec<_>>();
        others.sort_by(|a, b| a.path.cmp(&b.path));
        for module in others {
            let source = &module.program;
            program.types.extend(source.types.iter().cloned());
            program.enums.extend(source.enums.iter().cloned());
            program
                .module_vars
                .extend(source.module_vars.iter().cloned());
            program
                .module_consts
                .extend(source.module_consts.iter().cloned());
            program.declares.extend(source.declares.iter().cloned());
            program.delegates.extend(source.delegates.iter().cloned());
            program.interfaces.extend(source.interfaces.iter().cloned());
            program.classes.extend(source.classes.iter().cloned());
            program.procedures.extend(source.procedures.iter().cloned());
            program.functions.extend(source.functions.iter().cloned());
            program.properties.extend(source.properties.iter().cloned());
        }
        program.merge_partial_classes();
        Ok(Self { project, program })
    }
}

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub name: String,
    pub path: PathBuf,
    pub program: Program,
    pub imports: Vec<ResolvedImport>,
    pub file_id: FileId,
}

#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub module: usize,
    pub requested: String,
    pub qualifier: String,
    pub implicit: bool,
    pub span: Span,
}

pub fn load_project(entry_path: impl AsRef<Path>) -> Result<Project, (Diagnostic, SourceMap)> {
    let mut loader = ModuleLoader::default();
    match loader.load(entry_path.as_ref(), &mut Vec::new()) {
        Ok(entry) => Ok(Project {
            entry,
            modules: loader.modules,
            source_map: loader.source_map,
        }),
        Err(err) => Err((err, loader.source_map)),
    }
}

#[derive(Default)]
struct ModuleLoader {
    modules: Vec<LoadedModule>,
    by_path: HashMap<PathBuf, usize>,
    source_map: SourceMap,
}

impl ModuleLoader {
    fn load(&mut self, path: &Path, stack: &mut Vec<PathBuf>) -> Result<usize, Diagnostic> {
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("valo"))
        {
            return Err(Diagnostic::new(
                DiagnosticCode::MODULE_NOT_FOUND,
                "Valo source files must use the .valo extension; exported VBA modules are not supported",
                None,
            ));
        }
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
            let mut chain = String::new();
            let mut found = false;
            for entry in stack {
                if entry == &canonical {
                    found = true;
                }
                if found {
                    if chain.is_empty() {
                        chain.push_str(&module_name(entry));
                    } else {
                        chain.push_str("\n  -> ");
                        chain.push_str(&module_name(entry));
                    }
                }
            }
            chain.push_str("\n  -> ");
            chain.push_str(&module_name(&canonical));

            return Err(Diagnostic::new(
                DiagnosticCode::IMPORT_CYCLE,
                format!("Import cycle detected:\n\n{}", chain),
                None,
            )
            .with_help(
                "remove one import in the cycle or move shared declarations into a separate module",
            ));
        }
        if let Some(index) = self.by_path.get(&canonical).copied() {
            return Ok(index);
        }

        stack.push(canonical.clone());
        let source_content = read_source_file(&canonical)
            .map_err(|message| Diagnostic::new(DiagnosticCode::MODULE_NOT_FOUND, message, None))?;

        let name = module_name(&canonical);
        let file_id = self
            .source_map
            .add(canonical.display().to_string(), source_content.clone());
        let program = parse_source_with_id(&source_content, file_id)
            .map_err(|err| add_import_notes(err, &name, stack))?;

        let index = self.modules.len();
        self.by_path.insert(canonical.clone(), index);
        self.modules.push(LoadedModule {
            name,
            path: canonical.clone(),
            program,
            imports: Vec::new(),
            file_id,
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
                implicit: false,
                span: import.span,
            });
        }
        self.modules[index].imports = resolved;
        stack.pop();
        Ok(index)
    }
}

fn add_import_notes(mut err: Diagnostic, module_name: &str, stack: &[PathBuf]) -> Diagnostic {
    if stack.len() > 1 {
        err = err.with_note(format!("while parsing imported module `{module_name}`"));
        for importer in stack.iter().rev().skip(1) {
            err = err.with_note(format!("imported from {}", importer.display()));
        }
    }
    err
}

fn read_source_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|err| format!("Module '{}' could not be read: {err}", path.display()))?;
    let decoded = decode_source_bytes(&bytes)
        .ok_or_else(|| format!("Could not decode source file `{}`", path.display()))?;
    Ok(normalize_line_endings(&decoded))
}

fn decode_source_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8(bytes[3..].to_vec()).ok();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16(&bytes[2..], true);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16(&bytes[2..], false);
    }
    String::from_utf8(bytes.to_vec()).ok()
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Option<String> {
    let (pairs, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    let units: Vec<u16> = pairs
        .iter()
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes(*pair)
            } else {
                u16::from_be_bytes(*pair)
            }
        })
        .collect();
    String::from_utf16(&units).ok()
}

fn normalize_line_endings(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn module_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_string()
}

fn resolve_case_insensitive_path(
    base_dir: &Path,
    relative_path: &Path,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut current_paths = vec![base_dir.to_path_buf()];

    for component in relative_path.components() {
        let std::path::Component::Normal(part) = component else {
            for p in &mut current_paths {
                p.push(component);
            }
            continue;
        };

        let part_lower = part.to_string_lossy().to_ascii_lowercase();
        let mut next_paths = Vec::new();

        for dir in current_paths {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    if name.to_string_lossy().to_ascii_lowercase() == part_lower {
                        next_paths.push(entry.path());
                    }
                }
            }
        }
        current_paths = next_paths;
        if current_paths.is_empty() {
            break;
        }
    }
    Ok(current_paths)
}

fn resolve_import_path(current: &Path, import: &ImportDecl) -> Result<PathBuf, Diagnostic> {
    let dir = current.parent().unwrap_or_else(|| Path::new("."));
    let path_str = &import.module;
    let mut candidates = Vec::new();

    if path_str.to_ascii_lowercase().ends_with(".valo") {
        candidates.push(PathBuf::from(path_str));
    } else {
        let path = path_str.replace(".", "/");
        candidates.push(PathBuf::from(format!("{}.valo", path)));
        candidates.push(PathBuf::from(format!("{}/index.valo", path)));
    }

    let mut matches = Vec::new();
    for candidate in candidates {
        if let Ok(paths) = resolve_case_insensitive_path(dir, &candidate) {
            matches.extend(paths);
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
