//! Project/package manifest support.
//!
//! This is intentionally a small TOML subset parser for the first package
//! foundation. It keeps Valo independent of registry/package-manager concerns
//! while giving the CLI and future tooling a stable project identity contract.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::runtime::{Diagnostic, DiagnosticCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    pub root: PathBuf,
    pub name: String,
    pub version: String,
    pub entrypoint: PathBuf,
    pub authors: Vec<String>,
    pub target_platforms: Vec<String>,
    pub dependencies: BTreeMap<String, String>,
}

impl PackageManifest {
    pub fn entrypoint_path(&self) -> PathBuf {
        if self.entrypoint.is_absolute() {
            self.entrypoint.clone()
        } else {
            self.root.join(&self.entrypoint)
        }
    }
}

pub fn discover_manifest(start: impl AsRef<Path>) -> Result<Option<PackageManifest>, Diagnostic> {
    let start = start.as_ref();
    let manifest_path = if start.is_file()
        && start
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("valo.toml"))
    {
        Some(start.to_path_buf())
    } else if start.is_dir() {
        find_manifest_upward(start)
    } else {
        start.parent().and_then(find_manifest_upward)
    };

    manifest_path.map(load_manifest).transpose()
}

pub fn resolve_entrypoint(path: impl AsRef<Path>) -> Result<PathBuf, Diagnostic> {
    let path = path.as_ref();
    if path.is_dir()
        || path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("valo.toml"))
    {
        let manifest = discover_manifest(path)?.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::MODULE_NOT_FOUND,
                format!("No valo.toml manifest found near '{}'", path.display()),
                None,
            )
        })?;
        return Ok(manifest.entrypoint_path());
    }

    Ok(path.to_path_buf())
}

pub fn load_manifest(path: impl AsRef<Path>) -> Result<PackageManifest, Diagnostic> {
    let path = path.as_ref();
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let source = fs::read_to_string(path).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::MODULE_NOT_FOUND,
            format!("Could not read manifest '{}': {err}", path.display()),
            None,
        )
    })?;
    parse_manifest(&source, root)
}

fn find_manifest_upward(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let candidate = dir.join("valo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

fn parse_manifest(source: &str, root: PathBuf) -> Result<PackageManifest, Diagnostic> {
    let mut table = String::new();
    let mut name = None;
    let mut version = None;
    let mut entrypoint = None;
    let mut authors = Vec::new();
    let mut target_platforms = Vec::new();
    let mut dependencies = BTreeMap::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            table = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return manifest_error(line_index, "Expected key = value in valo.toml");
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match table.as_str() {
            "" | "package" => match key.as_str() {
                "name" => name = Some(parse_string(value, line_index)?),
                "version" => version = Some(parse_string(value, line_index)?),
                "entrypoint" => entrypoint = Some(PathBuf::from(parse_string(value, line_index)?)),
                "authors" => authors = parse_string_array(value, line_index)?,
                "compatibility" | "compatibility_mode" => {
                    return manifest_error(
                        line_index,
                        "Compatibility modes have been removed; remove this setting and use .valo sources",
                    );
                }
                "target_platforms" | "targets" => {
                    target_platforms = parse_string_array(value, line_index)?;
                }
                _ => {}
            },
            "dependencies" => {
                dependencies.insert(key, parse_string(value, line_index)?);
            }
            _ => {}
        }
    }

    Ok(PackageManifest {
        root,
        name: name.ok_or_else(|| manifest_diag(0, "valo.toml is missing package.name"))?,
        version: version.unwrap_or_else(|| "0.1.0".to_string()),
        entrypoint: entrypoint.unwrap_or_else(|| PathBuf::from("main.valo")),
        authors,
        target_platforms,
        dependencies,
    })
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut previous = '\0';
    for (index, ch) in line.char_indices() {
        if ch == '"' && previous != '\\' {
            in_string = !in_string;
        }
        if ch == '#' && !in_string {
            return &line[..index];
        }
        previous = ch;
    }
    line
}

fn parse_string(value: &str, line_index: usize) -> Result<String, Diagnostic> {
    let trimmed = value.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        Ok(trimmed[1..trimmed.len() - 1].replace("\\\"", "\""))
    } else {
        manifest_error(line_index, "Expected string value")
    }
}

fn parse_string_array(value: &str, line_index: usize) -> Result<Vec<String>, Diagnostic> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return manifest_error(line_index, "Expected string array value");
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|part| parse_string(part.trim(), line_index))
        .collect()
}

fn manifest_error<T>(line_index: usize, message: &str) -> Result<T, Diagnostic> {
    Err(manifest_diag(line_index, message))
}

fn manifest_diag(line_index: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::PACKAGE_MANIFEST,
        format!("{} at valo.toml:{}", message.into(), line_index + 1),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obsolete_compatibility_settings_are_rejected() {
        for key in ["compatibility", "compatibility_mode"] {
            for mode in ["native", "vba", "mixed"] {
                let source = format!("[package]\nname = \"app\"\n{key} = \"{mode}\"\n");
                let error = parse_manifest(&source, PathBuf::from(".")).unwrap_err();
                assert!(
                    error
                        .to_string()
                        .contains("Compatibility modes have been removed")
                );
            }
        }
    }

    #[test]
    fn ordinary_manifest_needs_no_compatibility_setting() {
        let manifest = parse_manifest("[package]\nname = \"app\"\n", PathBuf::from(".")).unwrap();
        assert_eq!(manifest.entrypoint, PathBuf::from("main.valo"));
    }
}
