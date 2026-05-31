use crate::backend::interpreter::run;
use crate::backend::interpreter::tests::helpers::{parse_and_validate, run_source};
use crate::frontend::parser::Parser;
use crate::frontend::semantics::validate;
use std::fs;

#[test]
fn property_let_allows_omitted_byval_like_vba() {
    let output = run_source(
        r#"
Class User
    Private m_Name As String

    Public Property Let Name(value As String)
        m_Name = value
    End Property

    Public Property Get Name() As String
        Name = m_Name
    End Property
End Class

Sub Main()
    Dim u As New User()
    u.Name = "Valo"
    Console.WriteLine(u.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn declare_parameters_accept_vba_keyword_like_names() {
    let program = parse_and_validate(
        r#"
Private Declare PtrSafe Function MFStartup Lib "mfplat" (
    ByVal Version As Long,
    ByVal dwFlags As Long
) As Long

Private Declare PtrSafe Function KeywordNames Lib "kernel32" (
    ByVal String As Long,
    ByVal Error As Long,
    ByVal Lib As Long
) As Long

Sub Main()
End Sub
"#,
    )
    .unwrap();

    assert_eq!(program.declares.len(), 2);
    assert_eq!(program.declares[0].params[0].name, "Version");
    assert_eq!(program.declares[1].params[0].name, "String");
}

#[test]
fn environ_builtin_supports_string_suffix_and_numeric_lookup() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine("missing=" & Environ$("VALO_TEST_ENV_THAT_SHOULD_NOT_EXIST"))
    Console.WriteLine("zero=" & Environ(0))
End Sub
"#,
    );

    assert_eq!(output, vec!["missing=", "zero="]);
}

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
        "Imports Ansi\nImports Wide\nSub Main()\nConsole.WriteLine(AnsiText())\nConsole.WriteLine(WideText())\nEnd Sub\n",
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
            Console.WriteLine(vbUseCompareOption)
            Console.WriteLine(vbDatabaseCompare)
            Console.WriteLine(vbString)
            Console.WriteLine(vbArray)
            Console.WriteLine(vbLongPtr)
            Console.WriteLine(VbMethod)
            Console.WriteLine(VbGet)
            Console.WriteLine(VbLet)
            Console.WriteLine(VbSet)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec!["0", "1", "-1", "2", "8", "8192", "26", "1", "2", "4", "8"]
    );
}

#[test]
fn generic_vba_runtime_constants_have_vba_values() {
    let source = r#"
        Sub Main()
            Console.WriteLine(Len(vbNullString))
            Console.WriteLine(Asc(vbCr) & "," & Asc(vbLf) & "," & Len(vbCrLf))
            Console.WriteLine(vbNewLine = vbCrLf)
            Console.WriteLine(Asc(vbTab))
            Console.WriteLine(Asc(vbBack) & "," & Asc(vbFormFeed) & "," & Asc(vbVerticalTab) & "," & Asc(vbNullChar))
            Console.WriteLine(vbTrue & "," & vbFalse & "," & vbUseDefault)
            Console.WriteLine(vbObjectError)
            Console.WriteLine(vbGeneralDate & "," & vbLongDate & "," & vbShortDate & "," & vbLongTime & "," & vbShortTime)
            Console.WriteLine(vbUseSystemDayOfWeek & "," & vbSunday & "," & vbMonday & "," & vbTuesday & "," & vbWednesday & "," & vbThursday & "," & vbFriday & "," & vbSaturday)
            Console.WriteLine(vbUseSystem & "," & vbFirstJan1 & "," & vbFirstFourDays & "," & vbFirstFullWeek)
            Console.WriteLine(vbNormal & "," & vbReadOnly & "," & vbHidden & "," & vbSystem & "," & vbVolume & "," & vbDirectory & "," & vbArchive & "," & vbAlias)
            Console.WriteLine(vbUpperCase & "," & vbLowerCase & "," & vbProperCase & "," & vbWide & "," & vbNarrow & "," & vbKatakana & "," & vbHiragana & "," & vbUnicode & "," & vbFromUnicode)
            Console.WriteLine(vbHide & "," & vbNormalFocus & "," & vbMinimizedFocus & "," & vbMaximizedFocus & "," & vbNormalNoFocus & "," & vbMinimizedNoFocus)
            Console.WriteLine(vba.vbCrLf = VBCRLF)
            Console.WriteLine(vbcrlf = VbCrLf)
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec![
            "0",
            "13,10,2",
            "True",
            "9",
            "8,12,11,0",
            "-1,0,-2",
            "-2147221504",
            "0,1,2,3,4",
            "0,1,2,3,4,5,6,7",
            "0,1,2,3",
            "0,1,2,4,8,16,32,64",
            "1,2,3,4,8,16,32,64,128",
            "0,1,2,3,4,6",
            "True",
            "True",
        ]
    );
}

#[test]
fn msgbox_and_vartype_constants_have_vba_values() {
    let source = "
        Sub Main()
            Console.WriteLine(vbOKOnly & \",\" & vbOKCancel & \",\" & vbAbortRetryIgnore & \",\" & vbYesNoCancel & \",\" & vbYesNo & \",\" & vbRetryCancel)
            Console.WriteLine(vbCritical & \",\" & vbQuestion & \",\" & vbExclamation & \",\" & vbInformation)
            Console.WriteLine(vbDefaultButton1 & \",\" & vbDefaultButton2 & \",\" & vbDefaultButton3 & \",\" & vbDefaultButton4)
            Console.WriteLine(vbApplicationModal & \",\" & vbSystemModal & \",\" & vbMsgBoxHelpButton & \",\" & vbMsgBoxSetForeground & \",\" & vbMsgBoxRight & \",\" & vbMsgBoxRtlReading)
            Console.WriteLine(vbOK & \",\" & vbCancel & \",\" & vbAbort & \",\" & vbRetry & \",\" & vbIgnore & \",\" & vbYes & \",\" & vbNo)
            Console.WriteLine(vbEmpty & \",\" & vbNull & \",\" & vbInteger & \",\" & vbLong & \",\" & vbSingle & \",\" & vbDouble & \",\" & vbCurrency & \",\" & vbDate)
            Console.WriteLine(vbString & \",\" & vbObject & \",\" & vbError & \",\" & vbBoolean & \",\" & vbVariant & \",\" & vbDataObject & \",\" & vbDecimal & \",\" & vbByte & \",\" & vbLongLong & \",\" & vbUserDefinedType & \",\" & vbArray)
            Console.WriteLine(VarType(\"hello\") = vbString)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec![
            "0,1,2,3,4,5",
            "16,32,48,64",
            "0,256,512,768",
            "0,4096,16384,65536,524288,1048576",
            "1,2,3,4,5,6,7",
            "0,1,2,3,4,5,6,7",
            "8,9,10,11,12,13,14,17,20,36,8192",
            "True",
        ]
    );
}

#[test]
fn common_safe_vba_string_functions_work() {
    let source = r#"
        Sub Main()
            Console.WriteLine(Left$("abcdef", 2) & "," & Right$("abcdef", 3) & "," & Mid$("abcdef", 2, 3))
            Console.WriteLine(Trim$("  a  ") & "," & LTrim$("  b") & "," & RTrim$("c  "))
            Console.WriteLine(UCase$("ab") & "," & LCase$("CD"))
            Console.WriteLine(Replace("a-b-a", "a", "x"))
            Console.WriteLine(InStr("alphabet", "ph") & "," & InStr(3, "alphabet", "a") & "," & InStrRev("one two one", "one"))
            Console.WriteLine(Space$(3) & "x")
            Console.WriteLine(String$(3, "A") & "," & Chr$(65) & "," & Asc("A"))
            Console.WriteLine(Hex$(255) & "," & Oct$(8) & "," & Val("  -12.5x") & "," & Str(12))
            Console.WriteLine(Len("é") & "," & LenB("é"))
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec![
            "ab,def,bcd",
            "a,b,c",
            "AB,cd",
            "x-b-x",
            "3,5,9",
            "   x",
            "AAA,A,65",
            "FF,10,-12.5, 12",
            "1,2",
        ]
    );
}

#[test]
fn vba_string_literals_preserve_backslashes() {
    let source = r#"
        Sub Main()
            Dim result As String
            result = "a" & vbTab & "b"
            result = Replace(result, vbTab, "\t")
            Console.WriteLine(result)
            Console.WriteLine("C:\Temp\payload.json")
            Console.WriteLine("\n")
        End Sub
    "#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["a\\tb", "C:\\Temp\\payload.json", "\\n"]);
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

fn temp_test_path(name: &str) -> std::path::PathBuf {
    fn sanitize_component(value: &str) -> String {
        value
            .chars()
            .map(|ch| match ch {
                ':' | '\\' | '/' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                ch if ch.is_control() => '_',
                ch => ch,
            })
            .collect()
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_name = std::thread::current().name().unwrap_or("test").to_string();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "valo_{}_{}_{}_{}",
        sanitize_component(name),
        std::process::id(),
        sanitize_component(&thread_name),
        stamp
    ));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn valo_string(path: &std::path::Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

#[test]
fn vba_test_temp_paths_are_windows_safe() {
    let path = temp_test_path("bad:name\\with/slashes*and?quotes\"<>|.txt");
    let name = path.file_name().unwrap().to_string_lossy();
    for invalid in [':', '\\', '/', '*', '?', '"', '<', '>', '|'] {
        assert!(!name.contains(invalid), "{name} contained {invalid}");
    }
}

#[test]
fn vba_file_output_append_input_and_eof_work() {
    let path = temp_test_path("file_io.txt");
    let source = format!(
        r#"
Sub Main()
    Dim f As Integer
    Dim line As String
    f = FreeFile
    Open "{}" For Output As #f
    Print #f, "hello"
    Print #f, "A"; "B"
    Close #f
    Console.WriteLine(FreeFile())
    Open "{}" For Append As #1
    Print #1, "tail"
    Close #1
    Open "{}" For Input As #1
    Console.WriteLine(EOF(1))
    Line Input #1, line
    Console.WriteLine(line)
    Line Input #1, line
    Console.WriteLine(line)
    Line Input #1, line
    Console.WriteLine(line)
    Console.WriteLine(EOF(1))
    Close
    Kill "{}"
End Sub
"#,
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["1", "False", "hello", "AB", "tail", "True"]);
    assert!(!path.exists());
}

#[test]
fn vba_print_trailing_semicolon_and_comma_work() {
    let path = temp_test_path("print_format.txt");
    let source = format!(
        r#"
Sub Main()
    Dim line As String
    Open "{}" For Output As #1
    Print #1, "A";
    Print #1, "B"
    Print #1,
    Print #1, "C",
    Print #1, "D"
    Close #1
    Open "{}" For Input As #1
    Line Input #1, line
    Console.WriteLine(line)
    Line Input #1, line
    Console.WriteLine(line)
    Line Input #1, line
    Console.WriteLine(line)
    Close #1
    Kill "{}"
End Sub
"#,
        valo_string(&path),
        valo_string(&path),
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["AB", "", "C\tD"]);
    assert!(!path.exists());
}

#[test]
fn vba_binary_get_put_scalars_work() {
    let path = temp_test_path("binary_io.bin");
    let source = format!(
        r#"
Sub Main()
    Dim b As Byte
    Dim n As Long
    Dim d As Double
    Dim flag As Boolean
    b = 7
    n = 123456
    d = 1.5
    flag = True
    Open "{}" For Binary As #1
    Put #1, , b
    Put #1, , n
    Put #1, , d
    Put #1, , flag
    Console.WriteLine(LOF(1))
    b = 0
    n = 0
    d = 0
    flag = False
    Get #1, 1, b
    Get #1, 2, n
    Get #1, 6, d
    Get #1, 14, flag
    Console.WriteLine(b & "," & n & "," & d & "," & flag)
    Close #1
    Kill "{}"
End Sub
"#,
        valo_string(&path),
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["15", "7,123456,1.5,True"]);
    assert!(!path.exists());
}

#[test]
fn vba_binary_get_put_accept_unprefixed_file_numbers() {
    let path = temp_test_path("binary_io_unprefixed.bin");
    let source = format!(
        r#"
Sub Main()
    Dim b As Byte
    b = 9
    Open "{}" For Binary As #1
    Put 1, , b
    b = 0
    Get 1, 1, b
    Console.WriteLine(b)
    Close 1
    Kill "{}"
End Sub
"#,
        valo_string(&path),
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["9"]);
    assert!(!path.exists());
}

#[test]
fn vba_random_get_put_len_records_work() {
    let path = temp_test_path("random_io.dat");
    let source = format!(
        r#"
Sub Main()
    Dim recordText As String
    Open "{}" For Random As #1 Len = 6
    Put #1, 2, "two"
    Put #1, 1, "one"
    recordText = ""
    Get #1, 1, recordText
    Console.WriteLine(recordText)
    Get #1, 2, recordText
    Console.WriteLine(recordText)
    Console.WriteLine(LOF(1))
    Close #1
    Kill "{}"
End Sub
"#,
        valo_string(&path),
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["one", "two", "12"]);
    assert!(!path.exists());
}

#[test]
fn vba_access_clauses_are_enforced_and_locks_parse() {
    let path = temp_test_path("access_io.bin");
    std::fs::write(&path, [1_u8]).unwrap();
    let source = format!(
        r#"
Sub Main()
    Dim b As Byte
    b = 2
    Open "{}" For Binary Access Read Lock Read Write As #1
    Put #1, , b
End Sub
"#,
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let error = run(&program).unwrap_err().to_string();
    assert!(error.contains("read-only"));
    std::fs::remove_file(&path).unwrap();

    let path = temp_test_path("access_write_io.bin");
    let source = format!(
        r#"
Sub Main()
    Dim b As Byte
    Open "{}" For Binary Access Write Lock Write As #1
    Get #1, , b
End Sub
"#,
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let error = run(&program).unwrap_err().to_string();
    assert!(error.contains("write-only"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vba_open_statement_accepts_shared_and_optional_hash_file_number() {
    let path = temp_test_path("open_shared_io.bin");
    std::fs::write(&path, [7_u8]).unwrap();
    let source = format!(
        r#"
Sub Main()
    Dim fileNum As Integer
    Dim b As Byte
    fileNum = FreeFile()
    Open "{}" For Binary Access Read Shared As fileNum
    Get #fileNum, , b
    Close #fileNum
    Console.WriteLine(b)
End Sub
"#,
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["7"]);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn vba_open_statement_accepts_common_clause_combinations() {
    let path = temp_test_path("open_clause_matrix.bin");
    std::fs::write(&path, [1_u8, 2_u8]).unwrap();
    let source = format!(
        r#"
Sub Main()
    Dim b As Byte
    Open "{}" For Binary As #1
    Close #1
    Open "{}" For Binary Access Read As #1
    Close #1
    Open "{}" For Binary Access Write As #1
    Close #1
    Open "{}" For Binary Access Read Write As #1
    Close #1
    Open "{}" For Binary Shared As #1
    Close #1
    Open "{}" For Binary Access Read Shared As #1
    Close #1
    Open "{}" For Binary Access Read Lock Read As #1
    Close #1
    Open "{}" For Binary Access Read Lock Write As #1
    Close #1
    Open "{}" For Binary Access Read Write Shared As #1
    Close #1
    Console.WriteLine("ok")
End Sub
"#,
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["ok"]);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn vba_open_statement_rejects_duplicate_locking_clauses_clearly() {
    let source = r#"
Sub Main()
    Open "x.bin" For Binary Shared Lock Read As #1
End Sub
"#;
    let error = Parser::parse_source(source, crate::runtime::FileId::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("either Shared or one Lock clause"));
}

#[test]
fn vba_filedatetime_and_datetime_functions_work() {
    let path = temp_test_path("datetime.txt");
    std::fs::write(&path, "x").unwrap();
    let source = format!(
        r#"
Sub Main()
    Dim t As Double
    t = Timer()
    Console.WriteLine(t >= 0 And t < 86400)
    Console.WriteLine(TypeName(FileDateTime("{}")))
    Console.WriteLine(TypeName(#1/1/1970#))
    Console.WriteLine(Year(DateSerial(2024, 2, 29)))
    Console.WriteLine(Month(DateSerial(2024, 2, 29)))
    Console.WriteLine(Day(DateSerial(2024, 2, 29)))
    Console.WriteLine(Hour(TimeSerial(13, 14, 15)))
    Console.WriteLine(Minute(TimeSerial(13, 14, 15)))
    Console.WriteLine(Second(TimeSerial(13, 14, 15)))
    Console.WriteLine(Weekday(DateSerial(2024, 2, 25)))
    Console.WriteLine(MonthName(2, True))
    Console.WriteLine(WeekdayName(1, True))
End Sub
"#,
        valo_string(&path)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec![
            "True", "Date", "Date", "2024", "2", "29", "13", "14", "15", "1", "Feb", "Sun"
        ]
    );
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn vba_zero_arg_runtime_functions_work_without_parentheses() {
    let source = r#"
Sub Main()
    Randomize 1
    Console.WriteLine(Timer >= 0 And Timer < 86400)
    Console.WriteLine(TypeName(Now))
    Console.WriteLine(TypeName(Date))
    Console.WriteLine(TypeName(Time))
    Console.WriteLine(Rnd >= 0 And Rnd < 1)
    Debug.Print "debug output"
End Sub
"#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec!["True", "Date", "Date", "Date", "True", "debug output"]
    );
}

#[test]
fn typed_optional_parameters_may_omit_defaults_like_vba() {
    let source = r#"
Sub Main()
    Console.WriteLine(WasOmitted())
    Console.WriteLine(WasOmitted(#1/1/1970#))
End Sub

Function WasOmitted(Optional ByVal value As Date) As Boolean
    WasOmitted = IsMissing(value)
End Function
"#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["True", "False"]);
}

#[test]
fn object_reference_accepts_any_class_instance() {
    let output = run_source(
        r#"
Class Player
    Public Name As String
End Class

Sub Main()
    Dim p As New Player()
    p.Name = "Ada"
    Dim obj As Object
    Set obj = p
    Console.WriteLine(IsObject(obj))
    Console.WriteLine(TypeName(obj))
    Console.WriteLine(TypeOf obj Is Player)
    Set obj = Nothing
    Console.WriteLine(obj Is Nothing)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "Player", "True", "True"]);
}

#[test]
fn createobject_type_checks_as_object() {
    let source = r#"
Sub Main()
    Dim obj As Object
    Set obj = CreateObject("Scripting.Dictionary", "")
End Sub
"#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
}

#[test]
fn object_default_property_assignment_accepts_string_index_semantically() {
    let source = r#"
Sub Main()
    Dim obj As Object
    obj("Name") = "Valo"
    Console.WriteLine("Name: " & obj("Name"))
End Sub
"#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
}

#[cfg(not(windows))]
#[test]
fn createobject_reports_clear_non_windows_runtime_error() {
    let error = crate::backend::interpreter::tests::helpers::source_error(
        r#"
Sub Main()
    Dim obj As Object
    Set obj = CreateObject("Scripting.Dictionary")
End Sub
"#,
    );

    assert!(error.contains("CreateObject is only available on Windows COM/OLE Automation hosts"));
}

#[test]
fn imported_bas_optional_and_callable_resolution_work() {
    let dir = temp_test_path("import_optional");
    std::fs::create_dir(&dir).unwrap();
    let main = dir.join("main.valo");
    let module = dir.join("Helpers.bas");
    std::fs::write(
        &main,
        r#"
Imports Helpers

Sub Main()
    Console.WriteLine(AddDefault(5))
    Console.WriteLine(AddDefault(5, 7))
    SayValue
End Sub
"#,
    )
    .unwrap();
    std::fs::write(
        &module,
        r#"
Public Function AddDefault(ByVal value As Integer, Optional ByVal extra As Integer = 10) As Integer
    AddDefault = value + extra
End Function

Public Sub SayValue(Optional ByVal value As String = "ok")
    Console.WriteLine(value)
End Sub
"#,
    )
    .unwrap();

    let output = crate::run_file(&main).unwrap();
    assert_eq!(output, vec!["15", "12", "ok"]);
    std::fs::remove_file(main).unwrap();
    std::fs::remove_file(module).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn imported_cls_method_can_call_same_class_method() {
    let dir = temp_test_path("import_cls_calls");
    std::fs::create_dir(&dir).unwrap();
    let main = dir.join("main.valo");
    let class = dir.join("Worker.cls");
    std::fs::write(
        &main,
        r#"
Imports Worker

Sub Main()
    Dim w As New Worker
    Console.WriteLine(w.Run())
End Sub
"#,
    )
    .unwrap();
    std::fs::write(
        &class,
        r#"
VERSION 1.0 CLASS
BEGIN
END
Attribute VB_Name = "Worker"

Public Function Run() As String
    Run = Helper()
End Function

Private Function Helper() As String
    Helper = "called"
End Function
"#,
    )
    .unwrap();

    let output = crate::run_file(&main).unwrap();
    assert_eq!(output, vec!["called"]);
    std::fs::remove_file(main).unwrap();
    std::fs::remove_file(class).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn vba_input_write_lof_seek_and_name_work() {
    let path = temp_test_path("file_io_write.txt");
    let renamed = temp_test_path("file_io_renamed.txt");
    let source = format!(
        r#"
Sub Main()
    Dim itemText As String
    Dim number As Integer
    Dim flag As Boolean
    Open "{}" For Output As #1
    Write #1, "alpha", 42, True
    Console.WriteLine(Seek(1))
    Close #1
    Console.WriteLine(FileLen("{}") = LOFValue("{}"))
    Name "{}" As "{}"
    Open "{}" For Input As #1
    Input #1, itemText, number, flag
    Console.WriteLine(itemText & "," & number & "," & flag)
    Close #1
    Kill "{}"
End Sub

Function LOFValue(ByVal path As String) As Integer
    Open path For Input As #2
    LOFValue = LOF(2)
    Close #2
End Function
"#,
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&path),
        valo_string(&renamed),
        valo_string(&renamed),
        valo_string(&renamed)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["19", "True", "alpha,42,True"]);
    assert!(!path.exists());
    assert!(!renamed.exists());
}

#[test]
fn vba_dir_and_directory_functions_work() {
    let dir = temp_test_path("dir_io");
    std::fs::create_dir(&dir).unwrap();
    let one = dir.join("one.txt");
    let two = dir.join("two.txt");
    let hidden = dir.join(".hidden.txt");
    let bin = dir.join("skip.bin");
    std::fs::write(&one, "1").unwrap();
    std::fs::write(&two, "2").unwrap();
    std::fs::write(&hidden, "h").unwrap();
    std::fs::write(&bin, "3").unwrap();
    let subdir = dir.join("child");
    let made = dir.join("made");
    let source = format!(
        r#"
Sub Main()
    Dim first As String
    Dim second As String
    first = Dir("{}")
    second = Dir()
    Console.WriteLine(first <> "")
    Console.WriteLine(second <> "")
    Console.WriteLine(Dir("{}"))
    MkDir "{}"
    Console.WriteLine(Dir("{}", vbDirectory) <> "")
    Console.WriteLine(Dir("{}", vbHidden) <> "")
    RmDir "{}"
End Sub
"#,
        valo_string(&dir.join("*.txt")),
        valo_string(&dir.join("missing.txt")),
        valo_string(&made),
        valo_string(&made),
        valo_string(&dir.join(".hidden.txt")),
        valo_string(&made)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["True", "True", "", "True", "True"]);
    std::fs::remove_file(one).unwrap();
    std::fs::remove_file(two).unwrap();
    std::fs::remove_file(hidden).unwrap();
    std::fs::remove_file(bin).unwrap();
    let _ = std::fs::remove_dir(subdir);
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn vba_file_io_diagnostics_are_clear() {
    let missing = temp_test_path("missing_file.txt");
    let source = format!(
        r#"
Sub Main()
    Open "{}" For Input As #1
End Sub
"#,
        valo_string(&missing)
    );
    let program = Parser::parse_source(&source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let error = run(&program).unwrap_err().to_string();
    assert!(error.contains("For Input"));

    let source = r#"
Sub Main()
    Open "a.txt" For Output As #0
End Sub
"#;
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let error = run(&program).unwrap_err().to_string();
    assert!(error.contains("File number must be between 1 and 511"));
}

#[cfg(windows)]
#[test]
fn test_createobject_fso() {
    let output = run_source(
        r#"
Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")
    Console.WriteLine(TypeName(fso))
End Sub
"#,
    );
    assert_eq!(output, vec!["Scripting.FileSystemObject"]);
}

#[cfg(windows)]
#[test]
fn test_fso_getfolder_returned_object_type() {
    let output = run_source(
        r#"
Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")
    Dim folder As Object
    Set folder = fso.GetFolder(".")
    Console.WriteLine(TypeName(folder))
End Sub
"#,
    );
    assert_eq!(output, vec!["IFolder"]);
}

#[cfg(windows)]
#[test]
fn test_fso_enumeration() {
    let output = run_source(
        r#"
Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")
    Dim folder As Object
    Set folder = fso.GetFolder("C:\Windows")
    
    Dim count As Integer
    count = 0
    Dim subf As Object
    For Each subf In folder.SubFolders
        count = count + 1
        If count >= 1 Then Exit For
    Next
    
    if count > 0 then
        Console.WriteLine("Enumerate OK")
    else
        Console.WriteLine("No subfolders found")
    end if
End Sub
"#,
    );
    assert!(output[0] == "Enumerate OK" || output[0] == "No subfolders found");
}

#[cfg(windows)]
#[test]
fn test_com_dictionary() {
    let output = run_source(
        r#"
Sub Main()
    Dim dict As Object
    Set dict = CreateObject("Scripting.Dictionary")
    dict.Add "a", 1
    dict.Add "b", 2
    Console.WriteLine(dict.Item("a"))
    Console.WriteLine(dict.Exists("b"))
End Sub
"#,
    );
    assert_eq!(output, vec!["1", "True"]);
}
