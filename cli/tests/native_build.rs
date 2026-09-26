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
            Command::new(&output)
                .env("VALO_RUNTIME_ASSERT_CLEAN", "1")
                .status()
                .unwrap()
                .code(),
            Some(0),
            "{}",
            source.display()
        );
        std::fs::remove_file(output).unwrap();
    }
}

#[test]
fn cli_builds_and_executes_multifile_native_project() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = manifest.join("tests/fixtures/multifile/main.valo");
    let output = std::env::temp_dir().join(format!(
        "valo-cli-multifile-{}{}",
        std::process::id(),
        if cfg!(windows) { ".exe" } else { "" }
    ));
    let build = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(0));
    std::fs::remove_file(output).unwrap();
}

#[test]
fn native_build_accepts_mixed_source_option_settings() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let root = std::env::temp_dir().join(format!("valo-mixed-options-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let main = root.join("main.valo");
    let imported = root.join("Other.valo");
    std::fs::write(
        &main,
        "Option Strict Off\nImports Other\nFunction Main() As Integer\nReturn OtherValue() - 1\nEnd Function\n",
    )
    .unwrap();
    std::fs::write(
        &imported,
        "Option Strict On\nOption Infer Off\nPublic Function OtherValue() As Integer\nDim Value As Integer = 1\nReturn Value\nEnd Function\n",
    )
    .unwrap();
    let output_path = root.join(if cfg!(windows) {
        "mixed.exe"
    } else {
        "mixed.out"
    });
    let result = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&main)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output_path).status().unwrap().code(), Some(0));
    std::fs::remove_file(output_path).unwrap();
    std::fs::remove_file(main).unwrap();
    std::fs::remove_file(imported).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn native_string_comparison_preserves_imported_file_option_compare() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let root = std::env::temp_dir().join(format!("valo-string-options-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let main = root.join("main.valo");
    let imported = root.join("Other.valo");
    std::fs::write(&main, "Option Compare Binary\nImports Other\nFunction Main() As Integer\nIf Equal(\"A\", \"a\") Then\nReturn 0\nEnd If\nReturn 1\nEnd Function\n").unwrap();
    std::fs::write(&imported, "Option Compare Text\nPublic Function Equal(A As String, B As String) As Boolean\nReturn A = B\nEnd Function\n").unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("check")
        .arg(&main)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let built = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&main)
        .output()
        .unwrap();
    assert!(!built.status.success());
    assert!(
        String::from_utf8_lossy(&built.stderr).contains("Option Compare Text string comparisons"),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    std::fs::remove_file(main).unwrap();
    std::fs::remove_file(imported).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn native_sub_main_calls_another_sub_and_returns_zero() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let root = std::env::temp_dir().join(format!("valo-sub-entry-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.valo");
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program.out"
    });
    std::fs::write(&source, "Module Program\nSub Work()\nDim X As Integer = 1\nEnd Sub\nSub Main()\nWork()\nEnd Sub\nEnd Module\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_valo"))
        .args([
            "build",
            source.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(0));
    std::fs::remove_file(source).unwrap();
    std::fs::remove_file(output).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn native_entry_can_be_in_an_imported_source() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let root = std::env::temp_dir().join(format!("valo-imported-entry-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.valo");
    let imported = root.join("Entry.valo");
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program.out"
    });
    std::fs::write(
        &source,
        "Imports Entry\nFunction Helper() As Integer\nReturn 0\nEnd Function\n",
    )
    .unwrap();
    std::fs::write(&imported, "Sub Main()\nEnd Sub\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_valo"))
        .args([
            "build",
            source.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(0));
    for path in [source, imported, output] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn duplicate_native_main_is_diagnosed() {
    let root = std::env::temp_dir().join(format!("valo-duplicate-entry-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.valo");
    std::fs::write(
        &source,
        "Sub Main()\nEnd Sub\nFunction Main() As Integer\nReturn 0\nEnd Function\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&source)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Main"));
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn invalid_native_main_signature_is_diagnosed() {
    let root = std::env::temp_dir().join(format!("valo-invalid-entry-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.valo");
    std::fs::write(
        &source,
        "Function Main(X As Integer) As Integer\nReturn X\nEnd Function\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&source)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("native entry must"));
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn qualified_cross_file_symbols_and_types_execute_natively() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let root = std::env::temp_dir().join(format!("valo-qualified-native-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.valo");
    let a = root.join("A.valo");
    let b = root.join("B.valo");
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program.out"
    });
    let ir = root.join("program.ll");
    let ir_again = root.join("program-again.ll");
    std::fs::write(&a, "Namespace A\nStructure Vector\nPublic X As Integer\nEnd Structure\nFunction Value() As Integer\nReturn 10\nEnd Function\nEnd Namespace\n").unwrap();
    std::fs::write(&b, "Namespace B\nStructure Vector\nPublic X As Integer\nEnd Structure\nFunction Value() As Integer\nReturn 20\nEnd Function\nEnd Namespace\n").unwrap();
    std::fs::write(&source, "Imports A\nImports B\nFunction Main() As Integer\nDim VA As A.Vector\nDim VB As B.Vector\nVA.X = A.Value()\nVB.X = B.Value()\nReturn VA.X + VB.X - 30\nEnd Function\n").unwrap();
    let build = |kind: &str, destination: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_valo"))
            .arg("build")
            .arg(&source)
            .arg(kind)
            .arg("-o")
            .arg(destination)
            .output()
            .unwrap()
    };
    let result = build("--emit=llvm-ir", &ir);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = std::fs::read_to_string(&ir).unwrap();
    let symbols = text
        .lines()
        .filter(|line| line.starts_with("define i32 @valo_"))
        .collect::<Vec<_>>();
    assert_eq!(symbols.len(), 3, "{text}");
    assert_ne!(symbols[1], symbols[2]);
    let result = build("--emit=llvm-ir", &ir_again);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rebuilt = std::fs::read_to_string(&ir_again).unwrap();
    let rebuilt_symbols = rebuilt
        .lines()
        .filter(|line| line.starts_with("define i32 @valo_"))
        .collect::<Vec<_>>();
    assert_eq!(symbols, rebuilt_symbols);
    let result = build("--emit=exe", &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(0));
    for path in [source, a, b, output, ir, ir_again] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn qualified_overloads_in_one_namespace_execute_natively() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let root =
        std::env::temp_dir().join(format!("valo-qualified-overloads-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("main.valo");
    let a = root.join("A.valo");
    let b = root.join("B.valo");
    let output = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program.out"
    });
    std::fs::write(&a, "Namespace A\nFunction Value(X As Integer) As Integer\nReturn X + 1\nEnd Function\nFunction Value(X As Double) As Integer\nReturn 2\nEnd Function\nEnd Namespace\n").unwrap();
    std::fs::write(&b, "Namespace B\nFunction Value(X As Integer) As Integer\nReturn X + 10\nEnd Function\nEnd Namespace\n").unwrap();
    std::fs::write(&source, "Imports A\nImports B\nFunction Main() As Integer\nReturn A.Value(1) + A.Value(1.0) + B.Value(1) - 15\nEnd Function\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_valo"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(Command::new(&output).status().unwrap().code(), Some(0));
    for path in [source, a, b, output] {
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn unsupported_native_class_initializer_reports_diagnostic_not_a_panic() {
    if LlvmTools::discover().is_err() {
        return;
    }
    let source =
        std::env::temp_dir().join(format!("valo-cli-unsupported-{}.valo", std::process::id()));
    std::fs::write(
        &source,
        "Class Resource\nPublic Sub Initialize()\nEnd Sub\nEnd Class\nFunction Main() As Integer\nDim R As New Resource()\nReturn 0\nEnd Function",
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
        String::from_utf8_lossy(&output.stderr).contains("native Class constructor"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
