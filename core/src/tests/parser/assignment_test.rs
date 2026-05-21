use crate::frontend::parser::Parser;
use crate::runtime::FileId;

#[test]
fn test_function_name_assignment() {
    let source = r#"
        Function Soma(ByVal a As Integer, ByVal b As Integer) As Integer
            Soma = a + b
        End Function

        Sub Main()
            Console.WriteLine(Soma(10, 20))
        End Sub
    "#;
    let program = Parser::parse_source(source, FileId::default());
    assert!(program.is_ok(), "Failed to parse: {:?}", program.err());
}

#[test]
fn test_function_set_assignment() {
    let source = r#"
        Class MyClass
        End Class

        Function GetObj() As MyClass
            Set GetObj = New MyClass
        End Function

        Sub Main()
        End Sub
    "#;
    let program = Parser::parse_source(source, FileId::default());
    assert!(program.is_ok(), "Failed to parse: {:?}", program.err());
}
