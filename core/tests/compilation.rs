use std::path::PathBuf;

use valo_core::{load_project, modules::Compilation};

#[test]
fn compilation_loads_transitive_sources_once_and_keeps_file_spans() {
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cli/tests/fixtures/multifile/main.valo");
    let project = load_project(source).unwrap();
    assert_eq!(project.modules.len(), 3);
    let compilation = Compilation::for_native(&project).unwrap();
    assert_eq!(compilation.program.types.len(), 1);
    assert_eq!(compilation.program.functions.len(), 3);
    let span = compilation
        .program
        .functions
        .iter()
        .find(|function| function.name == "LengthSquared")
        .unwrap()
        .span;
    assert_ne!(span.file_id, project.modules[project.entry].file_id);
}

#[test]
fn raycaster_imported_interface_is_visible_to_project_semantics() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../game/raycaster/main.valo");
    let project = load_project(source).unwrap();
    Compilation::for_native(&project).unwrap();
    assert!(project.modules.iter().any(|module| {
        module
            .program
            .interfaces
            .iter()
            .any(|interface| interface.name == "IThing")
    }));
}
