use crate::backend::interpreter::run;
use crate::frontend::parser::Parser;
use crate::frontend::semantics::validate;
use crate::runtime::FileId;

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
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["30"]);
}
