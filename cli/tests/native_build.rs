use std::path::PathBuf;
use std::process::Command;

use valo_core::backend::llvm::LlvmTools;

#[test]
fn cli_builds_and_executes_all_native_examples() {
    if let Err(error) = LlvmTools::discover() {
        eprintln!("LLVM CLI integration test skipped: {error}");
        return;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = manifest.join("../examples/native");
    let mut sources = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "valo"))
        .collect::<Vec<_>>();
    sources.sort();
    assert!(sources.len() >= 4);
    for source in sources {
        let output = std::env::temp_dir().join(format!(
            "valo-cli-native-{}-{}{}",
            std::process::id(),
            source.file_stem().unwrap().to_string_lossy(),
            if cfg!(windows) { ".exe" } else { "" }
        ));
        let build = Command::new(env!("CARGO_BIN_EXE_valo"))
            .arg("build")
            .arg(&source)
            .arg("--release")
            .arg("-o")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "{}: {}",
            source.display(),
            String::from_utf8_lossy(&build.stderr)
        );
        assert_eq!(
            Command::new(&output).status().unwrap().code(),
            Some(0),
            "{}",
            source.display()
        );
        std::fs::remove_file(output).unwrap();
    }
}

#[test]
fn unsupported_native_class_reports_eligibility_not_a_panic() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let source =
        std::env::temp_dir().join(format!("valo-cli-unsupported-{}.valo", std::process::id()));
    std::fs::write(
        &source,
        "Class Resource\nEnd Class\nFunction Main() As Integer\nDim R As Resource\nReturn 0\nEnd Function",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&source)
        .output()
        .unwrap();
    std::fs::remove_file(&source).unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("native eligibility"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
