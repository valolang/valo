use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::run_file;

fn temp_project() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("valo_modules_{stamp}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &std::path::Path, name: &str, source: &str) {
    fs::write(dir.join(name), source).unwrap();
}

#[test]
fn imports_same_directory_module_with_alias_and_state() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Math As M
Import State

Sub Main()
    Console.WriteLine(M.Add(10, 20))
    State.Increment()
    State.Increment()
    Console.WriteLine(State.Count())
End Sub
"#,
    );
    write(
        &dir,
        "Math.valo",
        r#"
Public Function Add(ByVal Left As Integer, ByVal Right As Integer) As Integer
    Return Left + Right
End Function
"#,
    );
    write(
        &dir,
        "State.valo",
        r#"
Private Total As Integer

Public Sub Increment()
    Total = Total + 1
End Sub

Public Function Count() As Integer
    Return Total
End Function

Sub Main()
    Console.WriteLine(999)
End Sub
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["30".to_string(), "2".to_string()]
    );
}

#[test]
fn unqualified_imported_function_must_be_unambiguous() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import A
Import B

Sub Main()
    Console.WriteLine(Add(1, 2))
End Sub
"#,
    );
    write(
        &dir,
        "A.valo",
        "Public Function Add(ByVal A As Integer, ByVal B As Integer) As Integer\nReturn A + B\nEnd Function\n",
    );
    write(
        &dir,
        "B.valo",
        "Public Function Add(ByVal A As Integer, ByVal B As Integer) As Integer\nReturn A + B\nEnd Function\n",
    );

    let error = run_file(dir.join("main.valo")).unwrap_err();
    assert_eq!(error.code, crate::runtime::DiagnosticCode::AMBIGUOUS_IMPORT);
}

#[test]
fn qualified_imported_function_bypasses_unqualified_ambiguity() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import A
Import B

Sub Main()
    Console.WriteLine(A.Add(1, 2))
    Console.WriteLine(B.Add(10, 20))
End Sub
"#,
    );
    write(
        &dir,
        "A.valo",
        "Public Function Add(ByVal A As Integer, ByVal B As Integer) As Integer\nReturn A + B\nEnd Function\n",
    );
    write(
        &dir,
        "B.valo",
        "Public Function Add(ByVal A As Integer, ByVal B As Integer) As Integer\nReturn A + B\nEnd Function\n",
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["3".to_string(), "30".to_string()]
    );
}

#[test]
fn duplicate_import_alias_is_rejected_case_insensitively() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        "Import A As M\nImport B As m\n\nSub Main()\nEnd Sub\n",
    );
    write(&dir, "A.valo", "");
    write(&dir, "B.valo", "");

    let error = run_file(dir.join("main.valo")).unwrap_err();
    assert_eq!(error.code, crate::runtime::DiagnosticCode::DUPLICATE_IMPORT);
}

#[test]
fn import_resolution_is_case_insensitive() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        "Import math As M\n\nSub Main()\nConsole.WriteLine(M.Add(1, 2))\nEnd Sub\n",
    );
    write(
        &dir,
        "Math.valo",
        "Public Function Add(ByVal A As Integer, ByVal B As Integer) As Integer\nReturn A + B\nEnd Function\n",
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["3".to_string()]
    );
}

#[test]
fn private_imported_function_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Math

Sub Main()
    Console.WriteLine(Math.Hidden())
End Sub
"#,
    );
    write(
        &dir,
        "Math.valo",
        "Private Function Hidden() As Integer\nReturn 1\nEnd Function\n",
    );

    let error = run_file(dir.join("main.valo")).unwrap_err();
    assert_eq!(error.code, crate::runtime::DiagnosticCode::PRIVATE_ACCESS);
}

#[test]
fn imported_public_constant_is_qualified_and_private_constant_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Settings

Sub Main()
    Console.WriteLine(Settings.Answer)
    Console.WriteLine(Settings.Hidden)
End Sub
"#,
    );
    write(
        &dir,
        "Settings.valo",
        "Public Const Answer As Integer = 42\nPrivate Const Hidden As Integer = 7\n",
    );

    let error = run_file(dir.join("main.valo")).unwrap_err();
    assert_eq!(error.code, crate::runtime::DiagnosticCode::PRIVATE_ACCESS);
}

#[test]
fn missing_module_is_reported() {
    let dir = temp_project();
    write(&dir, "main.valo", "Import Missing\n\nSub Main()\nEnd Sub\n");

    let error = run_file(dir.join("main.valo")).unwrap_err();
    assert_eq!(error.code, crate::runtime::DiagnosticCode::MODULE_NOT_FOUND);
}

#[test]
fn import_cycle_is_reported() {
    let dir = temp_project();
    write(&dir, "main.valo", "Import A\n\nSub Main()\nEnd Sub\n");
    write(&dir, "A.valo", "Import main\n");

    let error = run_file(dir.join("main.valo")).unwrap_err();
    assert_eq!(error.code, crate::runtime::DiagnosticCode::IMPORT_CYCLE);
}
