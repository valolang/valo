//! Valo Core
//!
//! This crate contains the core logic for the Valo language, divided into three primary layers:
//!
//! 1. **Frontend**: Lexer, Parser, AST, Semantics, and Module loading.
//! 2. **Runtime**: Core value system, type names, and diagnostics.
//! 3. **Backend**: The execution engine (currently a tree-walking interpreter).

pub mod backend;
pub mod frontend;
pub mod runtime;

// Re-exports for compatibility and ease of use
pub use backend::interpreter::{self, Frame, Interpreter, run};
pub use frontend::ast::*;
pub use frontend::lexer::{self, Lexer, Token, TokenKind};
pub use frontend::modules::{self, Project, load_project};
pub use frontend::package::{
    CompatibilityMode, PackageManifest, discover_manifest, load_manifest, resolve_entrypoint,
};
pub use frontend::parser::{self, Parser, parse_source, parse_source_with_id};
pub use frontend::preprocessor;
pub use frontend::semantics::{
    self, FunctionId, MemberId, ModuleId, ProjectIndex, SymbolId, TypeId, build_project_index,
    validate, validate_project, validate_project_for_check, validate_snippet,
};
pub use runtime::*;

// Top-level convenience functions
pub use runtime::Diagnostic;

pub fn run_source(source: &str) -> Result<Vec<String>, Diagnostic> {
    let program = parse_source(source)?;
    semantics::validate(&program)?;
    run(&program)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn __clear_cache(_beg: *mut std::ffi::c_void, _end: *mut std::ffi::c_void) {}

pub fn run_file(path: impl AsRef<std::path::Path>) -> Result<Vec<String>, String> {
    let entrypoint = resolve_entrypoint(path).map_err(|err| {
        let map = runtime::SourceMap::new();
        err.render(&map)
    })?;
    let project = match load_project(entrypoint) {
        Ok(p) => p,
        Err((err, map)) => return Err(err.render(&map)),
    };
    if let Err(err) = semantics::validate_project(&project) {
        return Err(err.render(&project.source_map));
    }
    Interpreter::new()
        .run_project(&project)
        .map_err(|err| err.render(&project.source_map))
}
