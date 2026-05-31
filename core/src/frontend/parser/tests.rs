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
fn parses_generic_class_header_without_newline_error() {
    let source = r#"
Class Box(Of T)
    Public Value As T
End Class

Sub Main()
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.classes.len(), 1);
    assert_eq!(program.classes[0].name, "Box");
    assert_eq!(program.classes[0].type_params, vec!["T"]);
}

#[test]
fn parses_structure_implements() {
    let source = r#"
Public Interface IImprimivel
    Sub Imprimir()
End Interface

Public Structure DocumentoInfo
    Implements IImprimivel

    Public Property Codigo As Integer

    Public Sub Imprimir() Implements IImprimivel.Imprimir
        Console.WriteLine("Código: " & Codigo)
    End Sub
End Structure

Sub Main()
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.types.len(), 1);
    let structure = &program.types[0];
    assert_eq!(structure.name, "DocumentoInfo");
    assert_eq!(structure.implements.len(), 1);
    assert_eq!(structure.members.len(), 4); // Auto-property (field + Get + Let) + Sub
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
fn parses_keyword_named_vba_enum_members() {
    let source = r#"
Enum FETypeJ: DeleteShp = 0: Text = 1: End Enum

Sub Main()
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.enums[0].members[1].name, "Text");
}

#[test]
fn parses_keyword_named_member_access() {
    let source = r#"
Sub Main()
    ActivePresentation.SlideShowWindow.View.Next
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 1);
}

#[test]
fn parses_unary_rhs_after_multiplicative_operator() {
    let source = r#"
Sub Main()
    Dim x As Integer
    x = 4 * -1
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 2);
}

#[test]
fn parses_keyword_named_arguments() {
    let source = r#"
Sub Main()
    Set dlgOpen = Application.FileDialog(Type:=msoFileDialogFilePicker)
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 1);
}

#[test]
fn parses_inline_case_body_with_multiple_colon_statements() {
    let source = r#"
Sub Main()
    Dim i As Integer
    Select Case "u"
        Case "u": i = 1: i = i + 4
        Case Else: i = 0
    End Select
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 2);
}

#[test]
fn parses_enum_member_identifier_in_conditions_and_optional_defaults() {
    let source = r#"
Public Enum LineBreaks: NeverBreak = 0: AlwaysBreak = 1: BreakOnMain = 2: End Enum

Private Function GetInd(Optional Ignore As Boolean) As String
    If lb = AlwaysBreak Then
        GetInd = " "
    ElseIf lb = BreakOnMain Then
        GetInd = ""
    End If
End Function

Public Function StringJson(Optional LineBreaks As LineBreaks = AlwaysBreak) As String
End Function
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.functions.len(), 2);
}

#[test]
fn parses_string_builtin_call_despite_type_keyword() {
    let source = r#"
Sub Main()
    Console.WriteLine(String(4, " "))
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 1);
}

#[test]
fn does_not_treat_plain_colon_as_named_argument() {
    let source = r#"
Sub Main()
    Debug.Print value: value = 1
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    assert_eq!(program.procedures[0].body.len(), 2);
}

#[test]
fn visibility_defaults() {
    let source = r#"
Dim x As Integer
Public y As Integer
Private z As Integer
Sub Sub1()
End Sub
Public Sub Sub2()
End Sub
Private Sub Sub3()
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();

    // Dim x defaults to Private
    assert_eq!(
        program
            .module_vars
            .iter()
            .find(|v| v.name == "x")
            .unwrap()
            .visibility,
        Visibility::Private
    );
    // Public y is Public
    assert_eq!(
        program
            .module_vars
            .iter()
            .find(|v| v.name == "y")
            .unwrap()
            .visibility,
        Visibility::Public
    );
    // Private z is Private
    assert_eq!(
        program
            .module_vars
            .iter()
            .find(|v| v.name == "z")
            .unwrap()
            .visibility,
        Visibility::Private
    );

    // Sub Sub1 defaults to Public
    assert_eq!(
        program
            .procedures
            .iter()
            .find(|p| p.name == "Sub1")
            .unwrap()
            .visibility,
        Visibility::Public
    );
    // Public Sub Sub2 is Public
    assert_eq!(
        program
            .procedures
            .iter()
            .find(|p| p.name == "Sub2")
            .unwrap()
            .visibility,
        Visibility::Public
    );
    // Private Sub Sub3 is Private
    assert_eq!(
        program
            .procedures
            .iter()
            .find(|p| p.name == "Sub3")
            .unwrap()
            .visibility,
        Visibility::Private
    );
}

#[test]
fn option_statement_position() {
    let source = r#"
Option Explicit
Dim x As Integer
"#;
    assert!(Parser::parse_source(source, FileId::default()).is_ok());

    let source = r#"
Attribute VB_Name = "Module1"
Option Explicit
Dim x As Integer
"#;
    assert!(Parser::parse_source(source, FileId::default()).is_ok());

    let source = r#"
Dim x As Integer
Option Explicit
"#;
    let result = Parser::parse_source(source, FileId::default());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .message
            .contains("Option statements must appear before declarations")
    );
}

#[test]
fn class_member_visibility_defaults() {
    let source = r#"
Class MyClass
    Dim x As Integer
    Public y As Integer
    Private z As Integer
    Sub Sub1()
    End Sub
    Public Sub Sub2()
    End Sub
    Private Sub Sub3()
    End Sub
End Class
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();
    let members = &program.classes[0].members;

    fn find_field<'a>(members: &'a [ClassMember], name: &str) -> &'a ClassField {
        for m in members {
            if let ClassMember::Field(f) = m
                && f.name == name
            {
                return f;
            }
        }
        panic!("Field {} not found", name);
    }

    fn find_sub<'a>(members: &'a [ClassMember], name: &str) -> &'a ClassSub {
        for m in members {
            if let ClassMember::Sub(s) = m
                && s.procedure.name == name
            {
                return s;
            }
        }
        panic!("Sub {} not found", name);
    }

    assert_eq!(find_field(members, "x").visibility, Visibility::Private);
    assert_eq!(find_field(members, "y").visibility, Visibility::Public);
    assert_eq!(find_field(members, "z").visibility, Visibility::Private);

    assert_eq!(find_sub(members, "Sub1").visibility, Visibility::Public);
    assert_eq!(find_sub(members, "Sub2").visibility, Visibility::Public);
    assert_eq!(find_sub(members, "Sub3").visibility, Visibility::Private);
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

#[test]
fn test_keyword_as_parameter_name() {
    let source =
        "Function Test(base As Double, text As String, compare As Integer) As Double\nEnd Function";
    let program = Parser::parse_source(source, FileId::default());
    assert!(program.is_ok(), "Failed to parse: {:?}", program.err());
}

#[test]
fn test_option_private_module() {
    let source = "Option Private Module\nSub Main()\nEnd Sub";
    let program = Parser::parse_source(source, FileId::default());
    assert!(program.is_ok(), "Failed to parse: {:?}", program.err());
}
#[test]
fn test_module_level_property() {
    let source = r#"
        Private mValue As Integer

        Public Property Get Value() As Integer
            Value = mValue
        End Property

        Public Property Let Value(v As Integer)
            mValue = v
        End Property
    "#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.properties.len(), 2);
    assert_eq!(program.properties[0].name, "Value");
    assert_eq!(program.properties[0].kind, PropertyKind::Get);
    assert_eq!(program.properties[1].kind, PropertyKind::Let);
}

#[test]
fn test_module_level_visibility() {
    let source = r#"
        Public Sub PublicSub()
        End Sub

        Private Sub PrivateSub()
        End Sub

        Friend Sub FriendSub()
        End Sub

        Sub DefaultSub()
        End Sub
    "#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.procedures.len(), 4);
    assert_eq!(program.procedures[0].visibility, Visibility::Public);
    assert_eq!(program.procedures[1].visibility, Visibility::Private);
    assert_eq!(program.procedures[2].visibility, Visibility::Friend);
    assert_eq!(program.procedures[3].visibility, Visibility::Public);
}

#[test]
fn test_module_level_iterator() {
    let source = r#"
        Public Iterator Function MyIterator() As Integer
            Yield 1
            Yield 2
        End Function
    "#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.functions.len(), 1);
    assert!(program.functions[0].is_iterator);
}

#[test]
fn test_option_order() {
    let source = r#"
        Option Explicit
        Dim x As Integer
        Option Base 1
    "#;
    let error = Parser::parse_source(source, FileId::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Option statements must appear before declarations")
    );
}

#[test]
fn test_class_module_attributes() {
    let source = r#"
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "Class1"
Option Explicit
Private m_value As Integer
Public Sub Foo()
End Sub
"#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.classes.len(), 1);
    assert_eq!(program.classes[0].name, "Class1");
    assert!(program.option_explicit);
    assert_eq!(program.classes[0].members.len(), 2); // m_value and Foo
}

#[test]
fn parses_namespace_declaration() {
    let source = r#"
Namespace Game.Graphics

Public Class Sprite
End Class

End Namespace
"#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.namespace.as_deref(), Some("Game.Graphics"));
    assert_eq!(program.classes.len(), 1);
    assert_eq!(program.classes[0].name, "Game.Graphics.Sprite");
}

#[test]
fn parses_nested_namespace_declarations_as_qualified_namespace() {
    let source = r#"
Namespace Game
Namespace Graphics

Public Class Sprite
End Class

End Namespace
End Namespace
"#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.namespace.as_deref(), Some("Game.Graphics"));
    assert_eq!(program.classes.len(), 1);
    assert_eq!(program.classes[0].name, "Game.Graphics.Sprite");
}

#[test]
fn parses_module_block_as_module_level_declarations() {
    let source = r#"
Public Module MathTools
    Public Const Answer As Integer = 42

    Public Function Add(ByVal left As Integer, ByVal right As Integer) As Integer
        Add = left + right
    End Function
End Module
"#;
    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.module_consts.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "Add");
}

#[test]
fn parses_chained_member_access() {
    let source = r#"
Sub Main()
    a.b.c = 1
    a.b(1).c = 2
    a.b.Item(1).c = 3
    slide.Shapes.Item(2).TextFrame.TextRange.Text = "x"
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.procedures[0].body.len(), 4);
}

#[test]
fn parses_keyword_as_member_name() {
    let source = r#"
Sub Main()
    obj.Text = "hello"
    obj.Base = 1
    obj.Version = 2
    obj.Sub = 3
End Sub
"#;

    let program = Parser::parse_source(source, FileId::default()).unwrap();
    assert_eq!(program.procedures[0].body.len(), 4);
}
