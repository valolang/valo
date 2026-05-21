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
        let source_content = read_source_file(&canonical)
            .map_err(|message| Diagnostic::new(DiagnosticCode::MODULE_NOT_FOUND, message, None))?;

        let name = module_name(&canonical);
        let file_id = self.source_map.add(name.clone(), source_content.clone());
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
    String::from_utf8(bytes.to_vec())
        .ok()
        .or_else(|| Some(decode_windows_1252(bytes)))
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Option<String> {
    let chunks = bytes.chunks_exact(2);
    if !chunks.remainder().is_empty() {
        return None;
    }
    let units: Vec<u16> = chunks
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect();
    String::from_utf16(&units).ok()
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match *byte {
            0x80 => '\u{20AC}',
            0x82 => '\u{201A}',
            0x83 => '\u{0192}',
            0x84 => '\u{201E}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02C6}',
            0x89 => '\u{2030}',
            0x8A => '\u{0160}',
            0x8B => '\u{2039}',
            0x8C => '\u{0152}',
            0x8E => '\u{017D}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201C}',
            0x94 => '\u{201D}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02DC}',
            0x99 => '\u{2122}',
            0x9A => '\u{0161}',
            0x9B => '\u{203A}',
            0x9C => '\u{0153}',
            0x9E => '\u{017E}',
            0x9F => '\u{0178}',
            byte => byte as char,
        })
        .collect()
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
