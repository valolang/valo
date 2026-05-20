use super::helpers::{run_source, source_error};

#[test]
fn using_calls_dispose_on_success() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Sub Main()
    Using res As New Resource()
        Console.WriteLine("inside")
    End Using
End Sub
"#,
    );

    assert_eq!(output, vec!["inside", "disposed"]);
}

#[test]
fn using_existing_variable_calls_dispose() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Sub Main()
    Dim res As New Resource()
    Using res
        Console.WriteLine("inside")
    End Using
End Sub
"#,
    );

    assert_eq!(output, vec!["inside", "disposed"]);
}

#[test]
fn manual_dispose_still_works() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("manual")
    End Sub
End Class

Sub Main()
    Dim res As New Resource()
    res.Dispose()
End Sub
"#,
    );

    assert_eq!(output, vec!["manual"]);
}

#[test]
fn using_calls_dispose_on_runtime_error() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Sub Main()
    Try
        Using res As New Resource()
            Err.Raise 1001, "Test", "failure"
        End Using
    Catch ex As Error
        Console.WriteLine("caught " & ex.Message)
    End Try
End Sub
"#,
    );

    assert_eq!(output, vec!["disposed", "caught failure"]);
}

#[test]
fn using_respects_on_error_resume_next() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Sub Main()
    On Error Resume Next
    Using res As New Resource()
        Err.Raise 1001, "Test", "failure"
        Console.WriteLine("after raise")
    End Using
    Console.WriteLine("after using")
End Sub
"#,
    );

    assert_eq!(output, vec!["after raise", "disposed", "after using"]);
}

#[test]
fn using_calls_dispose_before_return() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Function Test() As String
    Using res As New Resource()
        Return "value"
    End Using
End Function

Sub Main()
    Console.WriteLine(Test())
End Sub
"#,
    );

    assert_eq!(output, vec!["disposed", "value"]);
}

#[test]
fn using_calls_dispose_before_exit_sub() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub
End Class

Sub Test()
    Using res As New Resource()
        Exit Sub
    End Using
End Sub

Sub Main()
    Test()
    Console.WriteLine("after")
End Sub
"#,
    );

    assert_eq!(output, vec!["disposed", "after"]);
}

#[test]
fn using_declaration_is_scoped_to_block() {
    let error = source_error(
        r#"
Class Resource
    Public Sub Dispose()
    End Sub
End Class

Sub Main()
    Using res As New Resource()
    End Using
    res.Dispose()
End Sub
"#,
    );

    assert!(error.contains("Variable 'res' is not declared"));
}

#[test]
fn using_rejects_scalar_target() {
    let error = source_error(
        r#"
Sub Main()
    Dim n As Integer
    Using n
    End Using
End Sub
"#,
    );

    assert!(error.contains("Using target must be a class instance"));
}

#[test]
fn using_rejects_object_without_dispose() {
    let error = source_error(
        r#"
Class Resource
End Class

Sub Main()
    Using res As New Resource()
    End Using
End Sub
"#,
    );

    assert!(error.contains("has no Dispose method"));
}

#[test]
fn using_rejects_dispose_with_parameters() {
    let error = source_error(
        r#"
Class Resource
    Public Sub Dispose(ByVal code As Integer)
    End Sub
End Class

Sub Main()
    Using res As New Resource()
    End Using
End Sub
"#,
    );

    assert!(error.contains("Dispose method used by Using must be parameterless"));
}

#[test]
fn dispose_and_terminate_can_coexist() {
    let output = run_source(
        r#"
Class Resource
    Public Sub Dispose()
        Console.WriteLine("disposed")
    End Sub

    Public Sub Terminate()
        Console.WriteLine("terminated")
    End Sub
End Class

Sub Main()
    Using res As New Resource()
        Console.WriteLine("inside")
    End Using
End Sub
"#,
    );

    assert_eq!(output, vec!["inside", "disposed", "terminated"]);
}
