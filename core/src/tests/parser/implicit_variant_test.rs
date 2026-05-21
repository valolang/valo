use crate::frontend::parser::Parser;
use crate::runtime::FileId;

#[test]
fn test_implicit_variant_function() {
    let source = r#"
        Function Soma(a, b)
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
fn test_implicit_variant_dim() {
    let source = r#"
        Sub Main()
            Dim x
            x = 42
            Console.WriteLine(x)
        End Sub
    "#;
    let program = Parser::parse_source(source, FileId::default());
    assert!(program.is_ok(), "Failed to parse: {:?}", program.err());
}

#[test]
fn test_implicit_variant_property() {
    let source = r#"
        Class MyClass
            Private mValue
            Property Get Value()
                Value = mValue
            End Property
            Property Let Value(v)
                mValue = v
            End Property
        End Class

        Sub Main()
            Dim obj As MyClass
            Set obj = New MyClass
            obj.Value = 100
            Console.WriteLine(obj.Value)
        End Sub
    "#;
    let program = Parser::parse_source(source, FileId::default());
    assert!(program.is_ok(), "Failed to parse: {:?}", program.err());
}
