use crate::backend::interpreter::run;
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
    let program = Parser::parse_source(source).unwrap();
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
    let program = Parser::parse_source(source).unwrap();
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
    let program = Parser::parse_source(source).unwrap();
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
    let program = Parser::parse_source(source).unwrap();
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
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["caught"]);
}
