#[cfg(test)]
mod tests {
    use crate::frontend::lexer::Lexer;
    use crate::frontend::parser::Parser;
    use crate::runtime::FileId;

    #[test]
    fn test_multiline_declare_parsing() {
        let source = r#"
Declare PtrSafe Function MessageBoxA Lib "user32" Alias "MessageBoxA" ( _
    ByVal hwnd As LongPtr, _
    ByVal lpText As String, _
    ByVal lpCaption As String, _
    ByVal uType As Long _
) As Long
"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens, FileId::default());
        let program = parser.parse_program().unwrap();
        assert_eq!(program.declares.len(), 1);
    }

    #[test]
    fn test_declare_symbol_registration() {
        let source = r#"
Declare PtrSafe Function puts Lib "libc" CDecl (ByVal text As String) As Long
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
Private Declare PtrSafe Function puts Lib "libc" CDecl (
ByVal value As String
) As Long

Private Sub Main()
    puts("Hello")
End Sub
"#;
        let program = crate::parse_source(source).unwrap();
        assert_eq!(program.declares.len(), 1);
        crate::validate(&program).expect("Validation failed");
    }
}
