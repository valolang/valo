use crate::backend::interpreter::run;
use crate::backend::interpreter::tests::helpers::{run_source, source_error};
use crate::frontend::parser::Parser;
use crate::frontend::semantics::validate;

#[test]
fn test_try_catch_success() {
    let source = "
        Sub Main()
            Try
                Console.WriteLine(\"try\")
            Catch ex As Error
                Console.WriteLine(\"catch\")
            Finally
                Console.WriteLine(\"finally\")
            End Try
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["try", "finally"]);
}

#[test]
fn test_try_catch_error() {
    let source = "
        Sub Main()
            Try
                Err.Raise 100, \"Source\", \"Description\"
            Catch ex As Error
                Console.WriteLine(\"error: \" & ex.Number & \" \" & ex.Description)
            Finally
                Console.WriteLine(\"finally\")
            End Try
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["error: 100 Description", "finally"]);
}

#[test]
fn test_try_finally_error_propagation() {
    let source = "
        Sub Main()
            Try
                Dangerous()
            Catch ex As Error
                Console.WriteLine(\"caught in main\")
            End Try
        End Sub

        Sub Dangerous()
            Try
                Err.Raise 100
            Finally
                Console.WriteLine(\"finally in dangerous\")
            End Try
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["finally in dangerous", "caught in main"]);
}

#[test]
fn test_debug_print() {
    let source = "
        Sub Main()
            Debug.Print \"hello\", 1, True
            Debug.Print(\"world\")
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["hello\t1\tTrue", "world"]);
}

#[test]
fn test_try_catch_optional_variable() {
    let source = "
        Sub Main()
            Try
                Err.Raise 1
            Catch
                Console.WriteLine(\"caught\")
            End Try
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["caught"]);
}

#[test]
fn async_function_and_await_expression_run_synchronously() {
    let output = run_source(
        r#"
Async Function FetchAsync(ByVal id As Integer) As String
    Return "item-" & id
End Function

Async Sub Main()
    Dim value As String
    value = Await FetchAsync(42)
    Console.WriteLine(value)
End Sub
"#,
    );

    assert_eq!(output, vec!["item-42"]);
}

#[test]
fn async_sub_and_async_method_allow_await_statement() {
    let output = run_source(
        r#"
Class Worker
    Public Async Sub Run()
        Await NoOp()
        Console.WriteLine("method")
    End Sub
End Class

Async Function NoOp() As Integer
    Return 0
End Function

Async Sub Main()
    Dim worker As New Worker()
    worker.Run()
    Await NoOp()
    Console.WriteLine("main")
End Sub
"#,
    );

    assert_eq!(output, vec!["method", "main"]);
}

#[test]
fn await_requires_async_context() {
    let statement_error = source_error(
        r#"
Async Sub NoOp()
End Sub

Sub Main()
    Await NoOp()
End Sub
"#,
    );
    assert!(statement_error.contains("Await is only allowed inside Async Sub or Async Function"));

    let expression_error = source_error(
        r#"
Async Function ValueAsync() As Integer
    Return 1
End Function

Sub Main()
    Dim value As Integer
    value = Await ValueAsync()
End Sub
"#,
    );
    assert!(expression_error.contains("Await is only allowed inside Async Sub or Async Function"));
}
