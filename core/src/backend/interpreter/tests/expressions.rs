use crate::backend::interpreter::tests::helpers::*;

#[test]
fn evaluates_and_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(True And True)
    Console.WriteLine(True And False)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False"]);
}

#[test]
fn evaluates_or_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(False Or True)
    Console.WriteLine(False Or False)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False"]);
}

#[test]
fn evaluates_not_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(Not True)
    Console.WriteLine(Not False)
End Sub
"#,
    );

    assert_eq!(output, vec!["False", "True"]);
}

#[test]
fn evaluates_mod_result() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(10 Mod 3)
End Sub
"#,
    );

    assert_eq!(output, vec!["1"]);
}

#[test]
fn unary_numeric_literals_support_vba_suffixes_and_scientific_notation() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(-10#)
    Console.WriteLine(-10!)
    Console.WriteLine(-10&)
    Console.WriteLine(-10^)
    Console.WriteLine(-10@)
    Console.WriteLine(-1.5)
    Console.WriteLine(-.5)
    Console.WriteLine(-1E+3)
    Console.WriteLine(-2.5E-4)
    Console.WriteLine(+10#)
    Console.WriteLine(+1.5)
    Console.WriteLine(+.5)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "-10", "-10", "-10", "-10", "-10.0000", "-1.5", "-0.5", "-1000", "-0.00025", "10",
            "1.5", "0.5"
        ]
    );
}

#[test]
fn unary_numeric_operators_work_on_expressions_and_calls() {
    let output = run_source(
        r#"
Function Rand() As Double
    Rand = 1.25
End Function

Sub Main()
    Dim x As Double
    Dim y As Double
    x = 2.5
    y = 1.5
    Console.WriteLine(-(x + y))
    Console.WriteLine(-Rand())
    Console.WriteLine(--1)
    Console.WriteLine(+-1)
End Sub
"#,
    );

    assert_eq!(output, vec!["-4", "-1.25", "1", "-1"]);
}

#[test]
fn unary_numeric_precedence_matches_vba_power_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(-2 ^ 2)
    Console.WriteLine((-2) ^ 2)
    Console.WriteLine(2 ^ -2)
End Sub
"#,
    );

    assert_eq!(output, vec!["-4", "4", "0.25"]);
}

#[test]
fn unary_numeric_constants_and_radix_literals_fold() {
    let output = run_source(
        r#"
Const NEG As Long = -(5 + 2)
Const HEX As Long = -&H1
Const OCT As Long = -&O10

Sub Main()
    Console.WriteLine(NEG)
    Console.WriteLine(HEX)
    Console.WriteLine(OCT)
End Sub
"#,
    );

    assert_eq!(output, vec!["-7", "-1", "-8"]);
}

#[test]
fn unary_minus_rejects_non_numeric_values() {
    let diagnostic = source_diagnostic(
        r#"
Sub Main()
    Console.WriteLine(-"hello")
End Sub
"#,
    );

    assert_eq!(diagnostic.code.0, "V1100");
    assert!(diagnostic.message.contains("requires a numeric expression"));
}

#[test]
fn logical_operator_precedence() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(True Or False And False)
    Console.WriteLine(Not False And False)
    Console.WriteLine(Not (False And False))
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False", "True"]);
}

#[test]
fn like_operator_supports_vba_wildcards_and_option_compare() {
    let output = run_source(
        r#"
Option Compare Text

Sub Main()
    Console.WriteLine("Ada" Like "A*")
    Console.WriteLine("Ada" Like "A?a")
    Console.WriteLine("A7" Like "A#")
    Console.WriteLine("B" Like "[ABC]")
    Console.WriteLine("D" Like "[!ABC]")
    Console.WriteLine("ada" Like "A*")
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "True", "True", "True", "True", "True"]);
}

#[test]
fn compound_assignment_operators_update_in_place() {
    let output = run_source(
        r#"
Sub Main()
    Dim total As Long = 10
    total += 5
    Console.WriteLine(total)
    total -= 3
    Console.WriteLine(total)
    total *= 2
    Console.WriteLine(total)
    total \= 4
    Console.WriteLine(total)
    total ^= 2
    Console.WriteLine(total)

    Dim label As String = "Valo"
    label &= " 1.0"
    Console.WriteLine(label)
End Sub
"#,
    );

    assert_eq!(output, vec!["15", "12", "24", "6", "36", "Valo 1.0"]);
}

#[test]
fn compound_assignment_works_on_array_elements_and_fields() {
    let output = run_source(
        r#"
Class Counter
    Public Hits As Long
End Class

Sub Main()
    Dim slots(0 To 2) As Long
    slots(1) = 7
    slots(1) += 5
    Console.WriteLine(slots(1))

    Dim c As New Counter()
    c.Hits = 2
    c.Hits *= 10
    Console.WriteLine(c.Hits)
End Sub
"#,
    );

    assert_eq!(output, vec!["12", "20"]);
}

#[test]
fn shift_operators_preserve_the_left_operand_type_and_sign() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(1 << 10)
    Console.WriteLine(1024 >> 3)
    Console.WriteLine(-16 >> 2)

    Dim flags As Long = 3
    flags <<= 4
    Console.WriteLine(flags)
    flags >>= 2
    Console.WriteLine(flags)
End Sub
"#,
    );

    assert_eq!(output, vec!["1024", "128", "-4", "48", "12"]);
}

#[test]
fn shift_binds_tighter_than_comparison_and_looser_than_addition() {
    let output = run_source(
        r#"
Sub Main()
    ' 1 + 1 = 2, so 1 << 2 = 4, and 4 > 3 is True.
    Console.WriteLine(1 << 1 + 1 > 3)
End Sub
"#,
    );

    assert_eq!(output, vec!["True"]);
}

#[test]
fn interpolated_strings_render_values_escapes_and_formats() {
    let output = run_source(
        r#"
Sub Main()
    Dim name As String = "Valo"
    Dim count As Long = 3
    Console.WriteLine($"Hello, {name}! You have {count} items.")
    Console.WriteLine($"Sum: {count + 1}")
    Console.WriteLine($"Braces: {{literal}}")
    Console.WriteLine($"Quote: ""quoted"" and {name}")
    Console.WriteLine($"Nested call: {Mid(name, 2, 2)}")
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "Hello, Valo! You have 3 items.",
            "Sum: 4",
            "Braces: {literal}",
            "Quote: \"quoted\" and Valo",
            "Nested call: al",
        ]
    );
}

#[test]
fn interpolation_applies_format_specifiers_and_alignment() {
    let output = run_source(
        r#"
Sub Main()
    Dim price As Double = 12.5
    Dim name As String = "Valo"
    Console.WriteLine($"{price:0.00}")
    Console.WriteLine($"[{name,8}]")
    Console.WriteLine($"[{name,-8}]")
End Sub
"#,
    );

    assert_eq!(output, vec!["12.50", "[    Valo]", "[Valo    ]"]);
}

#[test]
fn an_empty_interpolation_hole_is_rejected() {
    let diagnostic = source_diagnostic(
        r#"
Sub Main()
    Console.WriteLine($"empty {}")
End Sub
"#,
    );

    assert_eq!(diagnostic.code, crate::runtime::DiagnosticCode::PARSE);
}

#[test]
fn ctype_converts_and_nameof_and_gettype_report_names() {
    let output = run_source(
        r#"
Class Dog
    Public Breed As String
End Class

Sub Main()
    Dim total As Long = 7
    Console.WriteLine(CType("42", Integer) + 1)
    Console.WriteLine(NameOf(total))
    Console.WriteLine(GetType(Dog))
    Console.WriteLine(GetType(Integer))
End Sub
"#,
    );

    assert_eq!(output, vec!["43", "total", "Dog", "Integer"]);
}

#[test]
fn directcast_reinterprets_and_trycast_yields_nothing_on_mismatch() {
    let output = run_source(
        r#"
Class Animal
    Public Name As String
End Class

Class Dog Inherits Animal
    Public Breed As String
End Class

Sub Main()
    Dim d As New Dog()
    d.Name = "Rex"
    Dim a As Animal = DirectCast(d, Animal)
    Console.WriteLine(a.Name)

    Dim plain As Animal = New Animal()
    Dim maybe As Dog = TryCast(plain, Dog)
    Console.WriteLine(maybe Is Nothing)
End Sub
"#,
    );

    assert_eq!(output, vec!["Rex", "True"]);
}

#[test]
fn directcast_rejects_an_unrelated_type() {
    let diagnostic = source_diagnostic(
        r#"
Class Animal
    Public Name As String
End Class

Class Machine
    Public Serial As String
End Class

Sub Main()
    Dim m As New Machine()
    Dim a As Animal = DirectCast(m, Animal)
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::TYPE_MISMATCH
    );
}

#[test]
fn a_member_can_be_reached_through_a_parenthesised_expression() {
    let output = run_source(
        r#"
Class Vector
    Public X As Double

    Public Shared Operator +(ByVal left As Vector, ByVal right As Vector) As Vector
        Dim sum As New Vector()
        sum.X = left.X + right.X
        Return sum
    End Operator

    Public Function Describe() As String
        Return "x=" & X
    End Function
End Class

Sub Main()
    Dim a As New Vector()
    a.X = 1
    Dim b As New Vector()
    b.X = 2

    Console.WriteLine((a + b).Describe())
End Sub
"#,
    );

    assert_eq!(output, vec!["x=3"]);
}

#[test]
fn a_query_filters_sorts_and_projects() {
    let output = run_source(
        r#"
Sub Main()
    Dim numbers As New Collection()
    numbers.Add(5)
    numbers.Add(2)
    numbers.Add(9)
    numbers.Add(7)

    Dim big = From n In numbers
              Where n > 2
              Order By n Descending
              Select n * 10

    Dim item As Variant
    For Each item In big
        Console.WriteLine(item)
    Next item
End Sub
"#,
    );

    assert_eq!(output, vec!["90", "70", "50"]);
}

#[test]
fn a_query_walks_an_array_and_can_take_skip_and_dedupe() {
    let output = run_source(
        r#"
Sub Main()
    Dim words(3) As String
    words(0) = "pear"
    words(1) = "fig"
    words(2) = "apple"
    words(3) = "fig"

    Dim item As Variant
    For Each item In From w In words Distinct Order By w Take 2
        Console.WriteLine(item)
    Next item

    For Each item In From w In words Skip 3
        Console.WriteLine("last " & item)
    Next item
End Sub
"#,
    );

    assert_eq!(output, vec!["apple", "fig", "last fig"]);
}

#[test]
fn a_query_reads_members_of_what_it_walks() {
    let output = run_source(
        r#"
Class Person
    Public Name_ As String
    Public Age As Long
End Class

Function Make(ByVal n As String, ByVal a As Long) As Person
    Dim p As New Person()
    p.Name_ = n
    p.Age = a
    Return p
End Function

Sub Main()
    Dim people As New Collection()
    people.Add(Make("Ada", 36))
    people.Add(Make("Bob", 24))
    people.Add(Make("Cy", 41))

    Dim item As Variant
    For Each item In From p In people Where p.Age > 30 Order By p.Age Select p.Name_
        Console.WriteLine(item)
    Next item
End Sub
"#,
    );

    assert_eq!(output, vec!["Ada", "Cy"]);
}

#[test]
fn a_range_variable_does_not_escape_its_query() {
    let error = source_error(
        r#"
Option Explicit

Sub Main()
    Dim numbers As New Collection()
    numbers.Add(1)
    Dim picked = From n In numbers Where n > 0
    Console.WriteLine(n)
End Sub
"#,
    );

    assert!(error.contains("Variable 'n' is not declared"));
}

#[test]
fn a_clause_that_reads_the_range_variable_cannot_follow_select() {
    let error = source_error(
        r#"
Sub Main()
    Dim numbers As New Collection()
    numbers.Add(1)
    Dim picked = From n In numbers Select n * 2 Where n > 0
End Sub
"#,
    );
    assert!(error.contains("which Select has replaced"));

    // Reshaping what the projection produced is fine on either side of it.
    let output = run_source(
        r#"
Sub Main()
    Dim numbers As New Collection()
    numbers.Add(1)
    numbers.Add(2)
    numbers.Add(1)

    Dim item As Variant
    For Each item In From n In numbers Select n * 10 Distinct Take 1
        Console.WriteLine(item)
    Next item
End Sub
"#,
    );
    assert_eq!(output, vec!["10"]);
}

#[test]
fn from_is_still_usable_where_it_is_not_a_query() {
    let output = run_source(
        r#"
Sub Main()
    Dim path_ As String = "a.txt"
    Dim n As Long
    n = FreeFile
    Console.WriteLine(n > 0)
End Sub
"#,
    );

    assert_eq!(output, vec!["True"]);
}

#[test]
fn an_operator_is_found_when_both_operands_are_bare_fields() {
    // Without Option Explicit a bare name used to type as Variant before the
    // enclosing class was consulted, so an operand that was a field of the
    // current class lost its type and the overloaded operator went unfound.
    let output = run_source(
        r#"
Public Structure Offset_
    Public Steps As Long

    Public Sub New(ByVal steps As Long)
        Me.Steps = steps
    End Sub

    Public Shared Operator +(ByVal left As Offset_, ByVal right As Offset_) As Offset_
        Return New Offset_(left.Steps + right.Steps)
    End Operator
End Structure

Public Class Walker
    Public Here As Offset_
    Public Stride As Offset_

    Public Sub Initialize()
        Here = New Offset_(1)
        Stride = New Offset_(2)
    End Sub

    Public Function Ahead() As Offset_
        Return Here + Stride
    End Function
End Class

Sub Main()
    Dim walker As New Walker()
    Console.WriteLine(walker.Ahead().Steps)
End Sub
"#,
    );

    assert_eq!(output, vec!["3"]);
}
