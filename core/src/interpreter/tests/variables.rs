use crate::interpreter::tests::helpers::*;

#[test]
fn runs_variables_and_console_writeline() {
    let output = run_source(
        r#"
Sub Main()
    Dim name As String
    Dim count As Integer
    name = "Valo"
    count = 40 + 2
    Console.WriteLine("Hello, " & name & " " & count)
End Sub
"#,
    );

    assert_eq!(output, vec!["Hello, Valo 42"]);
}

#[test]
fn variable_names_are_case_insensitive() {
    let output = run_source(
        r#"
Sub Main()
    Dim Name As String
    name = "Valo"
    Console.WriteLine(NAME)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn byref_sub_parameter_mutates_caller_variable() {
    let output = run_source(
        r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x)
    Console.WriteLine(x)
End Sub
"#,
    );

    assert_eq!(output, vec!["11"]);
}

#[test]
fn declares_type_dims_variable_and_reads_default_fields() {
    let output = run_source(
        r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
    );

    assert_eq!(output, vec!["", "0", "False"]);
}

#[test]
fn static_local_variables_persist_between_calls() {
    let output = run_source(
        r#"
Sub Counter()
    Static count As Integer
    count = count + 1
    Console.WriteLine(count)
End Sub

Sub Main()
    Counter
    Counter
    Counter
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2", "3"]);
}
