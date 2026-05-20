use super::helpers::{run_source, source_error};
use crate::frontend::parser::Parser;

#[test]
fn exported_class_envelope_ignored() {
    let source = r#"
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1
END
Attribute VB_Name = "Counter"

Public value As Integer
"#;
    let program = Parser::parse_source(source).unwrap();
    assert_eq!(program.classes.len(), 1);
    assert_eq!(program.classes[0].name, "Counter");
}

#[test]
fn class_terminate_runs_on_nothing() {
    let source = r#"
Class Logger
    Public Sub Class_Terminate()
        Console.WriteLine("terminated")
    End Sub
End Class

Sub Main()
    Dim l As New Logger
    Console.WriteLine("before")
    Set l = Nothing
    Console.WriteLine("after")
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["before", "terminated", "after"]);
}

#[test]
fn native_constructor_runs() {
    let source = r#"
Class Box
    Public value As Integer

    Public Sub New()
        value = 10
    End Sub
End Class

Sub Main()
    Dim box As New Box
    Console.WriteLine(box.value)
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["10"]);
}

#[test]
fn native_constructor_accepts_parameters() {
    let source = r#"
Class Box
    Public value As Integer

    Public Sub New(ByVal initial As Integer)
        value = initial
    End Sub
End Class

Sub Main()
    Dim box As Box
    Set box = New Box(42)
    Console.WriteLine(box.value)
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["42"]);
}

#[test]
fn duplicate_constructor_aliases_are_rejected() {
    let error = source_error(
        r#"
Class Box
    Public Sub New()
    End Sub

    Public Sub Class_Initialize()
    End Sub
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("duplicate constructor definitions"));
}

#[test]
fn native_terminate_runs_on_nothing() {
    let source = r#"
Class Logger
    Public Sub Terminate()
        Console.WriteLine("terminated")
    End Sub
End Class

Sub Main()
    Dim l As New Logger
    Console.WriteLine("before")
    Set l = Nothing
    Console.WriteLine("after")
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["before", "terminated", "after"]);
}

#[test]
fn duplicate_terminator_aliases_are_rejected() {
    let error = source_error(
        r#"
Class Logger
    Public Sub Terminate()
    End Sub

    Public Sub Class_Terminate()
    End Sub
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("duplicate terminator definitions"));
}

#[test]
fn constructor_block_is_rejected() {
    let error = source_error(
        r#"
Class Box
    Public value As Integer

    Public Constructor()
        value = 10
    End Constructor
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Expected class member"));
}

#[test]
fn end_constructor_is_rejected() {
    let error = source_error(
        r#"
Class Box
    Public Sub New()
    End Constructor
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Expected statement"));
}

#[test]
fn sub_new_outside_class_is_rejected() {
    let error = source_error(
        r#"
Sub New()
End Sub

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Sub New is only allowed inside Class"));
}

#[test]
fn terminate_block_is_rejected() {
    let error = source_error(
        r#"
Class Logger
    Public Terminate()
        Console.WriteLine("terminated")
    End Terminate
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Expected class member"));
}

#[test]
fn end_terminate_is_rejected() {
    let error = source_error(
        r#"
Class Logger
    Public Sub Terminate()
    End Terminate
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Expected statement"));
}

#[test]
fn parameterized_terminate_is_rejected() {
    let error = source_error(
        r#"
Class Logger
    Public Sub Terminate(ByVal code As Integer)
    End Sub
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Terminate methods cannot declare parameters"));
}

#[test]
fn class_initialize_and_class_terminate_compatibility_still_work() {
    let source = r#"
Class Logger
    Public value As Integer

    Private Sub Class_Initialize()
        value = 7
    End Sub

    Private Sub Class_Terminate()
        Console.WriteLine("terminated " & value)
    End Sub
End Class

Sub Main()
    Dim l As New Logger
    Console.WriteLine(l.value)
    Set l = Nothing
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["7", "terminated 7"]);
}

#[test]
fn class_terminate_runs_on_reassign() {
    let source = r#"
Class Logger
    Public name As String
    Public Sub Class_Terminate()
        Console.WriteLine("terminated " & me.name)
    End Sub
End Class

Sub Main()
    Dim l As New Logger
    l.name = "first"
    Set l = New Logger()
    l.name = "second"
    Console.WriteLine("end")
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["terminated first", "end", "terminated second"]);
}

#[test]
fn class_terminate_runs_when_out_of_scope() {
    let source = r#"
Class Logger
    Public Sub Class_Terminate()
        Console.WriteLine("terminated")
    End Sub
End Class

Sub Test()
    Dim l As New Logger
    Console.WriteLine("inside")
End Sub

Sub Main()
    Console.WriteLine("before")
    Test()
    Console.WriteLine("after")
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["before", "inside", "terminated", "after"]);
}

#[test]
fn class_terminate_runs_only_once_with_multiple_references() {
    let source = r#"
Class Logger
    Public Sub Class_Terminate()
        Console.WriteLine("terminated")
    End Sub
End Class

Sub Main()
    Dim a As New Logger
    Dim b As Logger
    Set b = a
    Set a = Nothing
    Console.WriteLine("a is nothing")
    Set b = Nothing
    Console.WriteLine("b is nothing")
End Sub
"#;
    let output = run_source(source);
    assert_eq!(output, vec!["a is nothing", "terminated", "b is nothing"]);
}

#[test]
fn class_terminate_with_array() {
    let source = r#"
Class Logger
    Public id As Integer
    Public Sub Class_Terminate()
        Console.WriteLine("terminated " & me.id)
    End Sub
End Class

Sub Main()
    Dim arr(1) As Logger
    Set arr(0) = New Logger()
    arr(0).id = 0
    Set arr(1) = New Logger()
    arr(1).id = 1
    
    Console.WriteLine("reassigning")
    Set arr(0) = Nothing
    Console.WriteLine("done")
End Sub
"#;
    let output = run_source(source);
    assert_eq!(
        output,
        vec!["reassigning", "terminated 0", "done", "terminated 1"]
    );
}
