use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::helpers::run_file_diagnostic;
use crate::run_file;
use crate::runtime::ffi_platform::platform_libc;

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
fn valo_toml_entrypoint_is_used_for_run_file() {
    let dir = temp_project();
    write(
        &dir,
        "valo.toml",
        r#"
[package]
name = "manifest-app"
version = "0.1.0"
entrypoint = "src/main.valo"
authors = ["Valo"]
compatibility = "mixed"
target_platforms = ["linux"]
"#,
    );
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.valo"),
        r#"
Sub Main()
    Console.WriteLine("manifest")
End Sub
"#,
    )
    .unwrap();

    assert_eq!(run_file(&dir).unwrap(), vec!["manifest"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn project_index_records_namespace_qualified_symbols() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Namespace Game.Graphics

Public Class Sprite
End Class

Sub Main()
End Sub

End Namespace
"#,
    );

    let project = crate::load_project(dir.join("main.valo")).unwrap();
    let index = crate::build_project_index(&project).unwrap();
    assert!(index.by_qualified_name.contains_key("game.graphics.sprite"));
    fs::remove_dir_all(dir).unwrap();
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

    let error = run_file_diagnostic(dir.join("main.valo"));
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

    let error = run_file_diagnostic(dir.join("main.valo"));
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

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE
    );
}

#[test]
fn imported_public_declare_is_callable_unqualified_and_qualified() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Native

Sub Main()
    Console.WriteLine(Native.MyLen("Valo"))
    Console.WriteLine(MyLen("Runtime"))
End Sub
"#,
    );
    write(
        &dir,
        "Native.valo",
        &format!(
            r#"
Public Declare PtrSafe Function MyLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long
"#,
            platform_libc()
        ),
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["4".to_string(), "7".to_string()]
    );
}

#[test]
fn imported_private_declare_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Native

Sub Main()
    Console.WriteLine(Native.HiddenLen("Valo"))
End Sub
"#,
    );
    write(
        &dir,
        "Native.valo",
        &format!(
            r#"
Private Declare PtrSafe Function HiddenLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long
"#,
            platform_libc()
        ),
    );

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE
    );
}

#[test]
fn declares_load_from_bas_and_cls_modules() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import NativeBas
Import NativeCls

Sub Main()
    Console.WriteLine(NativeBas.BasLen("Valo"))
    Console.WriteLine(NativeCls.ClsLen("Class"))
End Sub
"#,
    );
    write(
        &dir,
        "NativeBas.bas",
        &format!(
            r#"
Attribute VB_Name = "NativeBas"
Public Declare PtrSafe Function BasLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long
"#,
            platform_libc()
        ),
    );
    write(
        &dir,
        "NativeCls.cls",
        &format!(
            r#"
Attribute VB_Name = "NativeCls"
Public Declare PtrSafe Function ClsLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long
"#,
            platform_libc()
        ),
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["4".to_string(), "5".to_string()]
    );
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

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE
    );
}

#[test]
fn qualified_imported_class_construction_and_alias_work() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models As M

Sub Main()
    Dim user As M.User
    Set user = New M.User()
    Console.WriteLine(user.Id())
End Sub
"#,
    );
    write(
        &dir,
        "Models.valo",
        r#"
Public Class User
    Public Function Id() As Integer
        Return 7
    End Function
End Class
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["7".to_string()]
    );
}

#[test]
fn qualified_imported_type_records_work() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models

Sub Main()
    Dim p As Models.PersonRecord
    p.Name = "Ada"
    Console.WriteLine(p.Name)
End Sub
"#,
    );
    write(
        &dir,
        "Models.valo",
        r#"
Public Type PersonRecord
    Name As String
End Type
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["Ada".to_string()]
    );
}

#[test]
fn qualified_imported_structure_records_work() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models

Sub Main()
    Dim p As Models.Point
    p.X = 5
    p.Y = 6
    Console.WriteLine(p.X)
    Console.WriteLine(p.Y)
End Sub
"#,
    );
    write(
        &dir,
        "Models.valo",
        r#"
Public Structure Point
    X As Integer
    Y As Integer
End Structure
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["5".to_string(), "6".to_string()]
    );
}

#[test]
fn private_imported_structure_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models

Sub Main()
    Dim p As Models.Point
End Sub
"#,
    );
    write(
        &dir,
        "Models.valo",
        r#"
Private Structure Point
    X As Integer
End Structure
"#,
    );

    let error = run_file_diagnostic(dir.join("main.valo")).to_string();
    assert!(error.contains("Imported type 'Models.Point' is Private"));
}

#[test]
fn imported_structure_methods_properties_and_constructor_work() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Geometry

Sub Main()
    Dim p As New Geometry.Point(10, 20)
    Console.WriteLine(p.Sum())
    Console.WriteLine(p.IsZero)
    p.MoveBy(1, 2)
    Console.WriteLine(p.X)
    Console.WriteLine(p.Y)
End Sub
"#,
    );
    write(
        &dir,
        "Geometry.valo",
        r#"
Public Structure Point
    Public X As Integer
    Public Y As Integer

    Public Sub Constructor(ByVal x As Integer, ByVal y As Integer)
        X = x
        Y = y
    End Sub

    Public Function Sum() As Integer
        Return X + Y
    End Function

    Public Sub MoveBy(ByVal dx As Integer, ByVal dy As Integer)
        X = X + dx
        Y = Y + dy
    End Sub

    Public Property Get IsZero() As Boolean
        Return X = 0 And Y = 0
    End Property
End Structure
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec![
            "30".to_string(),
            "False".to_string(),
            "11".to_string(),
            "22".to_string()
        ]
    );
}

#[test]
fn qualified_imported_enum_type_and_member_work() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Enums As E

Sub Main()
    Dim day As E.Days
    day = E.Days.Friday
    Console.WriteLine(day)
End Sub
"#,
    );
    write(
        &dir,
        "Enums.valo",
        r#"
Public Enum Days
    Monday
    Friday = 5
End Enum
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["5".to_string()]
    );
}

#[test]
fn qualified_public_module_variable_persists_and_can_be_assigned() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import State

Sub Main()
    State.GlobalCounter = State.GlobalCounter + 1
    State.GlobalCounter = State.GlobalCounter + 1
    Console.WriteLine(State.GlobalCounter)
End Sub
"#,
    );
    write(&dir, "State.valo", "Public GlobalCounter As Integer\n");

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["2".to_string()]
    );
}

#[test]
fn private_imported_class_and_enum_are_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models As M

Sub Main()
    Dim item As M.Hidden
End Sub
"#,
    );
    write(&dir, "Models.valo", "Private Class Hidden\nEnd Class\n");

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE
    );

    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Enums As E

Sub Main()
    Dim value As E.Secret
End Sub
"#,
    );
    write(&dir, "Enums.valo", "Private Enum Secret\nValue\nEnd Enum\n");

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::MEMBER_IS_PRIVATE
    );
}

#[test]
fn unknown_qualified_symbol_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models

Sub Main()
    Console.WriteLine(Models.Missing)
End Sub
"#,
    );
    write(&dir, "Models.valo", "");

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::UNKNOWN_QUALIFIED_SYMBOL
    );
}

#[test]
fn invalid_qualified_new_target_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import Models As M

Sub Main()
    Dim p As M.PersonRecord
    Set p = New M.PersonRecord()
End Sub
"#,
    );
    write(
        &dir,
        "Models.valo",
        "Public Type PersonRecord\nName As String\nEnd Type\n",
    );

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(
        error.code,
        crate::runtime::DiagnosticCode::INVALID_QUALIFIED_ACCESS
    );
}

#[test]
fn missing_module_is_reported() {
    let dir = temp_project();
    write(&dir, "main.valo", "Import Missing\n\nSub Main()\nEnd Sub\n");

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(error.code, crate::runtime::DiagnosticCode::MODULE_NOT_FOUND);
}

#[test]
fn import_cycle_is_reported() {
    let dir = temp_project();
    write(&dir, "main.valo", "Import A\n\nSub Main()\nEnd Sub\n");
    write(&dir, "A.valo", "Import main\n");

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(error.code, crate::runtime::DiagnosticCode::IMPORT_CYCLE);
    assert!(error.message.contains("Import cycle detected"));
    assert!(error.message.contains("main\n  -> A\n  -> main"));
    assert!(
        error
            .helps
            .iter()
            .any(|help| help.contains("move shared declarations"))
    );
}
