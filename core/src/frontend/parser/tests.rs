use super::*;

#[test]
fn parses_main_with_if_and_while() {
    let source = r#"
Sub Main()
    Dim i As Integer
    i = 1
    While i < 3
        i = i + 1
    Wend
    If i = 3 Then
        Console.WriteLine("ok")
    Else
        Console.WriteLine("bad")
    End If
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures.len(), 1);
    assert_eq!(program.procedures[0].name, "Main");
    assert_eq!(program.procedures[0].body.len(), 4);
}

#[test]
fn parses_nested_if_and_while_blocks() {
    let source = r#"
Sub Main()
    Dim i As Integer
    i = 0

    While i < 2
        If i = 0 Then
            Console.WriteLine("first")
        Else
            While i < 2
                i = i + 1
            Wend
        End If
        i = i + 1
    Wend
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 3);
}

#[test]
fn parses_property_get_and_let() {
    let source = r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property
End Class

Sub Main()
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();
    let members = &program.classes[0].members;

    assert!(matches!(members[1], ClassMember::Property(_)));
    assert!(matches!(members[2], ClassMember::Property(_)));
}

#[test]
fn parses_vba_declare_frontend_metadata() {
    let source = r#"
Private Declare PtrSafe Function FindWindow Lib "user32" Alias "FindWindowA" (ByVal lpClassName As LongPtr, ByVal lpWindowName As Any) As LongLong
Public Declare PtrSafe Sub Sleep Lib "kernel32" (ByVal dwMilliseconds As Long)

Sub Main()
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.declares.len(), 2);
    assert!(program.declares[0].ptr_safe);
    assert_eq!(program.declares[0].lib, "user32");
    assert_eq!(program.declares[0].alias.as_deref(), Some("FindWindowA"));
    assert_eq!(program.declares[0].params.len(), 2);
    assert_eq!(
        program.declares[0].return_type,
        Some(crate::TypeName::Int64)
    );
    assert_eq!(program.declares[1].return_type, None);
}

#[test]
fn rejects_missing_statement_newline() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    Dim x As Integer x = 1
End Sub
"#,
        FileId::default(),
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Expected newline after statement")
    );
}

#[test]
fn reports_missing_end_if() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    If True Then
        Console.WriteLine("open")
End Sub
"#,
        FileId::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("Expected 'End If'"));
}

#[test]
fn reports_missing_wend() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    While True
        Console.WriteLine("open")
End Sub
"#,
        FileId::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("Expected 'Wend'"));
}

#[test]
fn reports_missing_next() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
End Sub
"#,
        FileId::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("Expected 'Next'"));
}

#[test]
fn reports_missing_end_sub() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    Console.WriteLine("open")
"#,
        FileId::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("Expected 'End Sub'"));
}

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
