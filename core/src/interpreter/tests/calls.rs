use crate::interpreter::tests::helpers::*;

#[test]
fn runs_simple_sub_call() {
    let output = run_source(
        r#"
Sub SayHello()
    Console.WriteLine("Hello")
End Sub

Sub Main()
    SayHello()
End Sub
"#,
    );

    assert_eq!(output, vec!["Hello"]);
}

#[test]
fn runs_sub_call_with_byval_parameter() {
    let output = run_source(
        r#"
Sub Show(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Main()
    Show("Valo")
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

