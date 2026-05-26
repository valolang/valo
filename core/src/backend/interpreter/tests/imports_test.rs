use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::run_file;

fn temp_project() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("valo_imports_test_{stamp}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &std::path::Path, name: &str, source: &str) {
    fs::write(dir.join(name), source).unwrap();
}

#[test]
fn imports_modern_syntax_works() {
    let dir = temp_project();
    write(
        &dir,
        "Math.valo",
        "Namespace Math\nPublic Function Add(a As Integer, b As Integer) As Integer\nReturn a + b\nEnd Function\nEnd Namespace",
    );
    write(
        &dir,
        "main.valo",
        "Imports Math\n\nSub Main()\nConsole.WriteLine(Math.Add(1, 2))\nEnd Sub\n",
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["3".to_string()]
    );
}

#[test]
fn imports_with_alias_works() {
    let dir = temp_project();
    write(
        &dir,
        "Math.valo",
        "Namespace Math\nPublic Function Add(a As Integer, b As Integer) As Integer\nReturn a + b\nEnd Function\nEnd Namespace",
    );
    write(
        &dir,
        "main.valo",
        "Imports M = Math\n\nSub Main()\nConsole.WriteLine(M.Add(1, 2))\nEnd Sub\n",
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["3".to_string()]
    );
}
