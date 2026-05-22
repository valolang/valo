use crate::backend::interpreter::tests::helpers::*;

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

#[test]
fn test_runtime_function_assignment() {
    let source = r#"
        Function Soma(ByVal a As Long, ByVal b As Long) As Long
            Soma = a + b
        End Function

        Sub Main()
            Console.WriteLine(Soma(10, 20))
        End Sub
    "#;
    let output = run_source(source);
    assert_eq!(output, vec!["30"]);
}
