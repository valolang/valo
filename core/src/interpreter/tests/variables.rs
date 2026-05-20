use crate::interpreter::tests::helpers::*;

#[test]
fn runs_variables_and_console_writeline() {
    let output = run_source(
        r#"
Sub Main()
    Dim name As String
    Dim count As Integer
    name = "Valo"
    count = 40 + 2
    Console.WriteLine("Hello, " & name & " " & count)
End Sub
"#,
    );

    assert_eq!(output, vec!["Hello, Valo 42"]);
}

#[test]
fn variable_names_are_case_insensitive() {
    let output = run_source(
        r#"
Sub Main()
    Dim Name As String
    name = "Valo"
    Console.WriteLine(NAME)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn byref_sub_parameter_mutates_caller_variable() {
    let output = run_source(
        r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x)
    Console.WriteLine(x)
End Sub
"#,
    );

    assert_eq!(output, vec!["11"]);
}

#[test]
fn declares_type_dims_variable_and_reads_default_fields() {
    let output = run_source(
        r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
    );

    assert_eq!(output, vec!["", "0", "False"]);
}

#[test]
fn declares_structure_dims_variable_and_reads_default_fields() {
    let output = run_source(
        r#"
Structure Point
    X As Integer
    Y As Integer
End Structure

Sub Main()
    Dim p As Point
    Console.WriteLine(p.X)
    Console.WriteLine(p.Y)
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "0"]);
}

#[test]
fn structure_field_assignment_read_and_value_copy_work() {
    let output = run_source(
        r#"
Structure Point
    X As Integer
    Y As Integer
End Structure

Sub Main()
    Dim a As Point
    Dim b As Point
    a.X = 10
    a.Y = 20
    b = a
    b.X = 99
    Console.WriteLine(a.X)
    Console.WriteLine(a.Y)
    Console.WriteLine(b.X)
End Sub
"#,
    );

    assert_eq!(output, vec!["10", "20", "99"]);
}

#[test]
fn static_local_variables_persist_between_calls() {
    let output = run_source(
        r#"
Sub Counter()
    Static count As Integer
    count = count + 1
    Console.WriteLine(count)
End Sub

Sub Main()
    Counter
    Counter
    Counter
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2", "3"]);
}

#[test]
fn declaration_initializers_and_local_inference_work() {
    let output = run_source(
        r#"
Sub Main()
    Dim name As String = "Valo"
    Dim age = 20
    Dim price = 10.5
    Dim active = True
    Dim big As UInt64 = 18446744073709551615
    Dim b As Byte = CByte(255)
    Dim dt As Date = CDate(1)
    Dim convertedLong = CLng(10)
    Dim convertedDouble = CDbl(10.5)
    Console.WriteLine(TypeName(name))
    Console.WriteLine(TypeName(age))
    Console.WriteLine(TypeName(price))
    Console.WriteLine(TypeName(active))
    Console.WriteLine(TypeName(big))
    Console.WriteLine(TypeName(b))
    Console.WriteLine(TypeName(dt))
    Console.WriteLine(TypeName(convertedLong))
    Console.WriteLine(TypeName(convertedDouble))
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "String", "Integer", "Double", "Boolean", "UInt64", "Byte", "Date", "Long", "Double"
        ]
    );
}

#[test]
fn multiple_declarations_use_vba_per_declarator_semantics() {
    let output = run_source(
        r#"
Sub Main()
    Dim a, b As Integer
    Dim c As Integer = 1, d = "x", e As Double = 2.5
    a = "variant"
    b = 2
    Console.WriteLine(TypeName(a))
    Console.WriteLine(TypeName(b))
    Console.WriteLine(c & d & e)
End Sub
"#,
    );

    assert_eq!(output, vec!["String", "Integer", "1x2.5"]);
}

#[test]
fn type_declaration_characters_map_to_vba_types() {
    let output = run_source(
        r#"
Sub Main()
    Dim i% = 10, l&, s$ = "Valo", x!, d#, c@
    Dim a, b%
    b = 2
    Console.WriteLine(TypeName(i))
    Console.WriteLine(TypeName(l))
    Console.WriteLine(TypeName(s))
    Console.WriteLine(TypeName(x))
    Console.WriteLine(TypeName(d))
    Console.WriteLine(TypeName(c))
    Console.WriteLine(TypeName(a))
    Console.WriteLine(TypeName(b))
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "Integer", "Long", "String", "Single", "Double", "Currency", "Empty", "Integer"
        ]
    );
}

#[test]
fn multiple_array_declarations_preserve_bounds_and_types() {
    let output = run_source(
        r#"
Sub Main()
    Dim a() As Integer, b() As String
    Dim matrix(1 To 3, 1 To 2) As Double, labels$()
    ReDim a(0 To 1)
    ReDim b(0 To 0)
    ReDim labels(0 To 0)
    a(0) = 7
    b(0) = "b"
    labels(0) = "label"
    matrix(1, 1) = 2.5
    Console.WriteLine(TypeName(a(0)))
    Console.WriteLine(TypeName(b(0)))
    Console.WriteLine(TypeName(matrix(1, 1)))
    Console.WriteLine(labels(0))
End Sub
"#,
    );

    assert_eq!(output, vec!["Integer", "String", "Double", "label"]);
}

#[test]
fn as_new_accepts_constructor_arguments() {
    let output = run_source(
        r#"
Class User
    Name As String

    Constructor(ByVal name As String)
        Me.Name = name
    End Constructor
End Class

Sub Main()
    Dim user As New User("Valo")
    Dim other = New User("Other")
    Console.WriteLine(user.Name)
    Console.WriteLine(other.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "Other"]);
}

#[test]
fn module_level_dim_declarations_support_initializers_and_multiples() {
    let output = run_source(
        r#"
Dim moduleName As String = "Valo", moduleCount = 2
Public Dim publicValue% = 3

Sub Main()
    Console.WriteLine(moduleName)
    Console.WriteLine(TypeName(moduleCount))
    Console.WriteLine(publicValue)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "Integer", "3"]);
}

#[test]
fn class_field_multiple_declarations_are_supported() {
    let output = run_source(
        r#"
Class Box
    Public first As Integer, second$, third
End Class

Sub Main()
    Dim box As New Box
    box.first = 1
    box.second = "two"
    box.third = True
    Console.WriteLine(TypeName(box.first))
    Console.WriteLine(TypeName(box.second))
    Console.WriteLine(TypeName(box.third))
End Sub
"#,
    );

    assert_eq!(output, vec!["Integer", "String", "Boolean"]);
}

#[test]
fn class_field_initializer_is_rejected_clearly() {
    let diagnostic = source_diagnostic(
        r#"
Class Box
    Public value As Integer = 1
End Class

Sub Main()
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::TYPE_MISMATCH
    );
}

#[test]
fn duplicate_name_in_multiple_declaration_is_rejected() {
    let diagnostic = source_diagnostic(
        r#"
Sub Main()
    Dim a As Integer, a As String
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION
    );
}

#[test]
fn type_declaration_character_conflict_is_rejected() {
    let diagnostic = source_diagnostic(
        r#"
Sub Main()
    Dim a% As String
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::TYPE_MISMATCH
    );
}

#[test]
fn as_new_with_initializer_is_rejected() {
    let diagnostic = source_diagnostic(
        r#"
Class User
End Class

Sub Main()
    Dim user As New User = New User()
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::TYPE_MISMATCH
    );
}
