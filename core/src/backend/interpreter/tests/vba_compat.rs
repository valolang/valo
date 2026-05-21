use crate::backend::interpreter::run;
use crate::frontend::parser::Parser;
use crate::frontend::semantics::validate;
use std::fs;

#[test]
fn test_callbyname() {
    let source = "
        Class Target
            Public Value As Integer
            Public Sub SetValue(ByVal v As Integer)
                Value = v
            End Sub
            Public Function GetValue() As Integer
                GetValue = Value
            End Function
        End Class

        Sub Main()
            Dim obj As New Target
            ' VbMethod = 1
            CallByName obj, \"SetValue\", 1, 42
            Console.WriteLine(obj.Value)
            
            ' VbGet = 2
            Console.WriteLine(CallByName(obj, \"GetValue\", 1))
            Console.WriteLine(CallByName(obj, \"Value\", 2))
            
            ' VbLet = 4
            CallByName obj, \"Value\", 4, 99
            Console.WriteLine(obj.Value)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["42", "42", "42", "99"]);
}

#[test]
fn new_without_parentheses_and_exponent_work() {
    let source = r#"
        Class Vec2
            Public X As Double
        End Class

        Function Make() As Vec2
            Set Make = New Vec2
        End Function

        Sub Main()
            Dim v As Vec2
            Set v = New Vec2
            v.X = 2 ^ 3
            Console.WriteLine(v.X)
            Set v = Make
            Console.WriteLine(TypeName(v))
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["8", "Vec2"]);
}

#[test]
fn class_level_const_and_multi_const_work() {
    let source = r#"
        Class Circle
            Private Const PI As Double = 3.5
            Public Function Diameter(ByVal r As Double) As Double
                Diameter = PI * r * 2
            End Function
        End Class

        Public Const A As Integer = 2, B As Integer = 4

        Sub Main()
            Dim c As New Circle
            Console.WriteLine(c.Diameter(A))
            Console.WriteLine(B)
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["14", "4"]);
}

#[test]
fn structure_field_defaults_initialize_records() {
    let source = r#"
        Structure Vector2
            Public X As Double = 1.5
            Public Y As String = "ok"
        End Structure

        Sub Main()
            Dim v As Vector2
            Console.WriteLine(v.X)
            Console.WriteLine(v.Y)
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["1.5", "ok"]);
}

#[test]
fn one_line_enum_body_parses_and_numbers_like_vba() {
    let source = r#"
        Enum FETypeJ: DeleteShp = 0: Rename = 1: Keep: End Enum
        Sub Main()
            Console.WriteLine(DeleteShp)
            Console.WriteLine(Rename)
            Console.WriteLine(Keep)
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn default_property_group_supports_indexed_assignment() {
    let source = r#"
        Class ListBox
            Private saved As String
            Public Property Get Item(ByVal index As Integer) As String
            Attribute Item.VB_UserMemId = 0
                Item = saved
            End Property
            Public Property Let Item(ByVal index As Integer, ByVal value As String)
            Attribute Item.VB_UserMemId = 0
                saved = value
            End Property
        End Class

        Sub Main()
            Dim list As New ListBox
            list(0) = "zero"
            Console.WriteLine(list(0))
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["zero"]);
}

#[test]
fn ansi_and_utf16_imports_decode() {
    let unique = format!(
        "valo_vba_compat_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).unwrap();
    let main_path = dir.join("main.valo");
    let ansi_path = dir.join("Ansi.bas");
    let utf16_path = dir.join("Wide.bas");

    fs::write(
        &main_path,
        "Import Ansi\nImport Wide\nSub Main()\nConsole.WriteLine(AnsiText())\nConsole.WriteLine(WideText())\nEnd Sub\n",
    )
    .unwrap();
    fs::write(
        &ansi_path,
        b"Function AnsiText() As String\nAnsiText = \"caf\xe9\"\nEnd Function\n",
    )
    .unwrap();
    let mut wide = vec![0xFF, 0xFE];
    for unit in "Function WideText() As String\nWideText = \"wide\"\nEnd Function\n".encode_utf16()
    {
        wide.extend(unit.to_le_bytes());
    }
    fs::write(&utf16_path, wide).unwrap();

    let output = crate::run_file(&main_path).unwrap();
    assert_eq!(output, vec!["café", "wide"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_vba_constants() {
    let source = "
        Sub Main()
            Console.WriteLine(vbBinaryCompare)
            Console.WriteLine(vbTextCompare)
            Console.WriteLine(vbString)
            Console.WriteLine(vbArray)
            Console.WriteLine(VbMethod)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["0", "1", "8", "8192", "1"]);
}

#[test]
fn test_random() {
    let source = "
        Sub Main()
            Randomize 123
            Dim r1 As Double
            r1 = Rnd()
            Randomize 123
            Dim r2 As Double
            r2 = Rnd()
            Console.WriteLine(r1)
            Console.WriteLine(r2)
            ' Deterministic seeding
            If r1 = r2 Then
                Console.WriteLine(\"matched\")
            End If
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output[2], "matched");
}

#[test]
fn test_vba_namespace() {
    let source = "
        Sub Main()
            Dim parts As Variant
            parts = VBA.Split(\"a,b,c\", \",\")
            Console.WriteLine(VBA.Join(parts, \"-\"))
            Console.WriteLine(VBA.TypeName(123))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["a-b-c", "Integer"]);
}

#[test]
fn test_isempty() {
    let source = "
        Sub Main()
            Dim v As Variant
            Console.WriteLine(IsEmpty(v))
            v = 1
            Console.WriteLine(IsEmpty(v))
            v = Empty
            Console.WriteLine(IsEmpty(v))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["True", "False", "True"]);
}

#[test]
fn test_return_modernization() {
    let source = "
        Function Test(ByVal x As Integer) As Integer
            If x > 10 Then
                Return x * 2
            End If
            Test = x + 1
        End Function

        Sub Main()
            Console.WriteLine(Test(15))
            Console.WriteLine(Test(5))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["30", "6"]);
}
