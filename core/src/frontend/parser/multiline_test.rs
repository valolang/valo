#[cfg(test)]
mod tests {
    use crate::frontend::lexer::Lexer;
    use crate::frontend::parser::Parser;
    use crate::runtime::FileId;

    #[test]
    fn test_multiline_declare_parsing() {
        let source = r#"
Declare Function MessageBoxA Lib "user32" Alias "MessageBoxA" ( _
    ByVal hwnd As Ptr, _
    ByVal lpText As String, _
    ByVal lpCaption As String, _
    ByVal uType As Int32 _
) As Int32
"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens, FileId::default());
        let program = parser.parse_program().unwrap();
        assert_eq!(program.declares.len(), 1);
    }

    #[test]
    fn test_declare_symbol_registration() {
        let source = r#"
Declare Function puts Lib "libc" CDecl (ByVal text As String) As Int32
Sub Main()
    Call puts("Hello")
End Sub
"#;
        let program = crate::parse_source(source).unwrap();
        crate::validate(&program).expect("Validation failed");
    }

    #[test]
    fn test_declare_parameters_accept_vba_line_breaks_without_continuation() {
        let source = r#"
Private Declare Function puts Lib "libc" CDecl (
ByVal value As String
) As Int32

Private Sub Main()
    puts("Hello")
End Sub
"#;
        let program = crate::parse_source(source).unwrap();
        assert_eq!(program.declares.len(), 1);
        crate::validate(&program).expect("Validation failed");
    }
}
