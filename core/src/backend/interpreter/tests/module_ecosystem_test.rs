use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::run_file;

fn temp_project() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("valo_module_ecosystem_{stamp}"));
    fs::create_dir_all(&dir).unwrap();

    // Create valo.toml to make it a valid project
    fs::write(
        dir.join("valo.toml"),
        r#"
[package]
name = "test-project"
version = "0.1.0"
entrypoint = "main.valo"
"#,
    )
    .unwrap();

    dir
}

fn write(dir: &std::path::Path, name: &str, source: &str) {
    fs::write(dir.join(name), source).unwrap();
}

#[test]
fn module_ecosystem_namespaces_and_aliasing() {
    let dir = temp_project();

    // Namespace: Game.Data -> Game/Data.valo
    fs::create_dir_all(dir.join("Game")).unwrap();
    write(
        &dir.join("Game"),
        "Data.valo",
        "Namespace Game.Data\nPublic Function GetId() As Integer\nReturn 101\nEnd Function\nEnd Namespace",
    );

    // Namespace: Game.UI -> Game/UI.valo
    write(
        &dir.join("Game"),
        "UI.valo",
        "Namespace Game.UI\nPublic Function GetName() As String\nReturn \"Hero\"\nEnd Function\nEnd Namespace",
    );

    // Main project using multiple imports and aliasing
    write(
        &dir,
        "main.valo",
        r#"
Imports Game.Data
Imports U = Game.UI

Sub Main()
    Console.WriteLine(GetId())
    Console.WriteLine(U.GetName())
End Sub
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["101".to_string(), "Hero".to_string()]
    );
}

#[test]
fn nested_module_resolution() {
    let dir = temp_project();
    fs::create_dir_all(dir.join("Utilities")).unwrap();
    write(
        &dir.join("Utilities"),
        "Math.valo",
        "Namespace Utilities.Math\nPublic Function Double(v As Integer) As Integer\nReturn v * 2\nEnd Function\nEnd Namespace",
    );

    write(
        &dir,
        "main.valo",
        r#"
Imports Utilities.Math

Sub Main()
    Console.WriteLine(Double(5))
End Sub
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["10".to_string()]
    );
}
