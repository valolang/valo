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
    assert!(
        index.by_qualified_name.contains_key("game.graphics.sprite"),
        "Missing key: game.graphics.sprite"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imports_same_directory_module_with_alias_and_state() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports Math As M
Imports State

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
Imports A
Imports B

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
Imports A
Imports B

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
fn imported_partial_classes_merge_across_modules() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports PersonFields
Imports PersonMethods

Sub Main()
    Dim p As New Person()
    p.Name = "Ada"
    p.SayHello()
End Sub
"#,
    );
    write(
        &dir,
        "PersonFields.valo",
        r#"
Public Partial Class Person
    Public Name As String
End Class
"#,
    );
    write(
        &dir,
        "PersonMethods.valo",
        r#"
Public Partial Class Person
    Public Sub SayHello()
        Console.WriteLine("Hello, " & Me.Name)
    End Sub
End Class
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["Hello, Ada".to_string()]
    );
}

#[test]
fn partial_class_merge_does_not_hide_non_partial_import_ambiguity() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports PersonFields
Imports PersonMethods
Imports OtherPerson

Sub Main()
    Dim p As New Person()
End Sub
"#,
    );
    write(
        &dir,
        "PersonFields.valo",
        "Public Partial Class Person\nPublic Name As String\nEnd Class\n",
    );
    write(
        &dir,
        "PersonMethods.valo",
        "Public Partial Class Person\nPublic Sub SayHello()\nEnd Sub\nEnd Class\n",
    );
    write(
        &dir,
        "OtherPerson.valo",
        "Public Class Person\nPublic Id As Integer\nEnd Class\n",
    );

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(error.code, crate::runtime::DiagnosticCode::AMBIGUOUS_IMPORT);
}

#[test]
fn duplicate_import_alias_is_rejected_case_insensitively() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        "Imports A As M\nImports B As m\n\nSub Main()\nEnd Sub\n",
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
        "Imports math As M\n\nSub Main()\nConsole.WriteLine(M.Add(1, 2))\nEnd Sub\n",
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
Imports Math

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
Imports Native

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
Public Declare Function MyLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long
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
Imports Native

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
Private Declare Function HiddenLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long
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
fn explicit_same_project_import_exposes_public_members() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports Hidden

Sub Main()
    Console.WriteLine(Hidden.Value())
End Sub
"#,
    );
    write(
        &dir,
        "Hidden.valo",
        r#"

Public Function Value() As String
    Value = "hidden"
End Function
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["hidden".to_string()]
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn loaded_but_unimported_module_is_not_an_ambient_unqualified_candidate() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports A
Imports C

Sub Main()
    Console.WriteLine(A.Start())
End Sub
"#,
    );
    write(
        &dir,
        "A.valo",
        r#"
Imports B

Public Function Start() As String
    Return Value()
End Function
"#,
    );
    write(
        &dir,
        "B.valo",
        r#"
Public Function Value() As String
    Return "b"
End Function
"#,
    );
    write(
        &dir,
        "C.valo",
        r#"
Public Function Value() As String
    Return "c"
End Function
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["b".to_string()]
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vba_compat_private_udts_resolve_inside_imported_module_runtime_state() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports RiffLike

Sub Main()
    Console.WriteLine(RiffLike.BufferIsInactive())
End Sub
"#,
    );
    write(
        &dir,
        "RiffLike.valo",
        r#"
Option Explicit

#If VBA7 Then
Private Type RiffBuffer
    Active As Boolean
End Type

Private Type RiffContext
    Buffers(0 To 1) As RiffBuffer
End Type
#Else
Private Type RiffBuffer
    Active As Boolean
End Type

Private Type RiffContext
    Buffers(0 To 1) As RiffBuffer
End Type
#End If

Private rCtx As RiffContext

Public Function BufferIsInactive() As Boolean
    BufferIsInactive = Not rCtx.Buffers(0).Active
End Function
"#,
    );

    assert_eq!(run_file(dir.join("main.valo")).unwrap(), vec!["True"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imported_vba_function_can_be_called_as_statement() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports RiffLike

Sub Main()
    RiffOpen()
    Console.WriteLine("done")
End Sub
"#,
    );
    write(
        &dir,
        "RiffLike.valo",
        r#"
Option Explicit

Private opened As Boolean

Public Function RiffOpen() As Boolean
    opened = True
    RiffOpen = opened
End Function
"#,
    );

    assert_eq!(run_file(dir.join("main.valo")).unwrap(), vec!["done"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imported_vba_record_field_array_assignment_persists() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports Buffers

Sub Main()
    Console.WriteLine(Buffers.Initialize())
    Console.WriteLine(Buffers.ReadFirst())
End Sub
"#,
    );
    write(
        &dir,
        "Buffers.valo",
        r#"
Private Type BufferState
    Values(0 To 1) As Single
End Type

Private state As BufferState

Public Function Initialize() As Boolean
    state.Values(0) = 1!
    Initialize = True
End Function

Public Function ReadFirst() As Single
    ReadFirst = state.Values(0)
End Function
"#,
    );

    assert_eq!(run_file(dir.join("main.valo")).unwrap(), vec!["True", "1"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imported_vba_addressof_resolves_private_current_module_callback() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports Callbacks

Sub Main()
    Console.WriteLine(Callbacks.MakePtr() <> 0)
End Sub
"#,
    );
    write(
        &dir,
        "Callbacks.valo",
        r#"
Private Function Dummy() As Long
    Dummy = 1
End Function

Public Function MakePtr() As Ptr
    MakePtr = AddressOf Dummy
End Function
"#,
    );

    assert_eq!(run_file(dir.join("main.valo")).unwrap(), vec!["True"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[cfg(windows)]
fn imported_vba_byref_any_preserves_record_shape_on_writeback() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports GuidModule

Sub Main()
    Console.WriteLine(GuidModule.ParseGuid())
End Sub
"#,
    );
    write(
        &dir,
        "GuidModule.valo",
        r#"
Option Explicit

Private Declare Function IIDFromString Lib "ole32" (ByVal lpsz As Ptr, ByRef lpiid As Any) As Int32

Private Type GUID
    Data1 As Int32
    Data2 As Int16
    Data3 As Int16
    Data4(0 To 7) As Byte
End Type

Public Function ParseGuid() As String
    Dim value As GUID
    IIDFromString StrPtr("{00000001-0002-0003-0405-060708090A0B}"), value
    ParseGuid = CStr(value.Data1) & ":" & CStr(value.Data2) & ":" & CStr(value.Data3) & ":" & CStr(value.Data4(0)) & ":" & CStr(value.Data4(7))
End Function
"#,
    );

    assert_eq!(run_file(dir.join("main.valo")).unwrap(), vec!["1:2:3:4:11"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imported_public_constant_is_qualified_and_private_constant_is_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports Settings

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
Imports Models As M

Sub Main()
    Dim user As M.User
    user = New M.User()
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
Imports Models

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
Imports Models

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
Imports Models

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
    eprintln!("ACTUAL ERROR: {}", error);
    assert!(error.contains("Imported type 'Models.Point' is Private"));
}

#[test]
fn imported_structure_methods_properties_and_constructor_work() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Imports Geometry

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

    Public Sub New(ByVal x As Integer, ByVal y As Integer)
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

    Public ReadOnly Property IsZero() As Boolean
        Get
        Return X = 0 And Y = 0
        End Get
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
Imports Enums As E

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
Imports State

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
Imports Models As M

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
Imports Enums As E

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
Imports Models

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
Imports Models As M

Sub Main()
    Dim p As M.PersonRecord
    p = New M.PersonRecord()
End Sub
"#,
    );
    write(
        &dir,
        "Models.valo",
        "Public Type PersonRecord\nName As String\nEnd Type\n",
    );

    // A UDT cannot be constructed with New. Project validation now checks
    // procedure bodies, so this is reported before the program runs rather than
    // surfacing later as a qualified-access failure.
    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(error.code, crate::runtime::DiagnosticCode::TYPE_MISMATCH);
    assert!(error.message.contains("cannot be constructed with New"));
}

#[test]
fn missing_module_is_reported() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        "Imports Missing\n\nSub Main()\nEnd Sub\n",
    );

    let error = run_file_diagnostic(dir.join("main.valo"));
    assert_eq!(error.code, crate::runtime::DiagnosticCode::MODULE_NOT_FOUND);
}

#[test]
fn import_cycle_is_reported() {
    let dir = temp_project();
    write(&dir, "main.valo", "Imports A\n\nSub Main()\nEnd Sub\n");
    write(&dir, "A.valo", "Imports main\n");

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

#[test]
fn an_imported_type_can_be_named_without_a_qualifier() {
    let dir = temp_project();
    write(
        &dir,
        "Shapes.valo",
        r#"
Public Class Thing
    Public N As Long
End Class
"#,
    );
    write(
        &dir,
        "main.valo",
        r#"
Imports Shapes

Interface IHolds
    Function Held() As Thing
End Interface

Class Holder Implements IHolds
    Public Item As Thing

    Public Function Held() As Thing Implements IHolds.Held
        Return Item
    End Function
End Class

Sub Main()
    Dim holder As New Holder()
    holder.Item = New Thing()
    holder.Item.N = 7
    Console.WriteLine(holder.Held().N)
End Sub
"#,
    );

    assert_eq!(run_file(dir.join("main.valo")).unwrap(), vec!["7"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_class_can_inherit_from_one_declared_in_the_same_imported_module() {
    let dir = temp_project();
    write(
        &dir,
        "Zoo.valo",
        r#"
Public Class Animal
    Public Name As String

    Public Overridable Function Speak() As String
        Return "..."
    End Function
End Class

Public Class Dog Inherits Animal
    Public Overrides Function Speak() As String
        Return Name & " says woof"
    End Function
End Class
"#,
    );
    write(
        &dir,
        "main.valo",
        r#"
Imports Zoo

Sub Main()
    Dim pet As New Dog()
    pet.Name = "Rex"
    Console.WriteLine(pet.Speak())
End Sub
"#,
    );

    assert_eq!(
        run_file(dir.join("main.valo")).unwrap(),
        vec!["Rex says woof"]
    );
    fs::remove_dir_all(dir).unwrap();
}
