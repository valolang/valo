use crate::backend::interpreter::run;
use crate::frontend::parser::Parser;
use crate::frontend::semantics::validate;

#[test]
fn test_numeric_ranges_and_overflow() {
    let source = "
        Sub Main()
            Dim b As Byte
            b = 255
            Console.WriteLine(b)
            
            Dim i As Integer
            i = 32767
            Console.WriteLine(i)
            
            Dim l As Long
            l = 2147483647
            Console.WriteLine(l)
            
            Dim i64 As Int64
            i64 = 9223372036854775807
            Console.WriteLine(i64)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec!["255", "32767", "2147483647", "9223372036854775807"]
    );
}

#[test]
fn test_mixed_arithmetic() {
    let source = "
        Sub Main()
            Dim i As Integer
            Dim d As Double
            i = 10
            d = 2.5
            Console.WriteLine(i + d)
            Console.WriteLine(i * d)
            
            Dim s As Single
            s = 1.5
            Console.WriteLine(i + s)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["12.5", "25", "11.5"]);
}

#[test]
fn test_conversions() {
    let source = "
        Sub Main()
            Dim v As Variant
            v = 123.456
            Console.WriteLine(CByte(v))
            Console.WriteLine(CInt(v))
            Console.WriteLine(CLng(v))
            Console.WriteLine(CDbl(v))
            Console.WriteLine(CLng(\"42\"))
            Console.WriteLine(CByte(\"&HAA\"))
            
            Dim d As Date
            d = CDate(46152.0) ' Example serial date
            Console.WriteLine(TypeName(d))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec!["123", "123", "123", "123.456", "42", "170", "Date"]
    );
}

#[test]
fn test_byte_array() {
    let source = "
        Sub Main()
            Dim data() As Byte
            ReDim data(0 To 1)
            data(0) = 65
            data(1) = 66
            Console.WriteLine(data(0))
            Console.WriteLine(data(1))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["65", "66"]);
}

#[test]
fn byte_square_bracket_array_syntax_is_rejected() {
    let source = "
        Sub Main()
            Dim data As Byte[]
        End Sub
    ";
    let error = Parser::parse_source(source, crate::runtime::FileId::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("Square-bracket array type syntax is not supported"));
}

#[test]
fn test_unsigned_types() {
    let source = "
        Sub Main()
            Dim u32 As UInt32
            u32 = 4294967295
            Console.WriteLine(u32)
            
            Dim u64 As UInt64
            u64 = 18446744073709551615
            Console.WriteLine(u64)
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["4294967295", "18446744073709551615"]);
}

#[test]
fn test_ptr_foundation() {
    let source = "
        Sub Main()
            Dim p As Ptr
            p = 0
            Console.WriteLine(TypeName(p))
            
            Dim f As FuncPtr
            f = 0
            Console.WriteLine(TypeName(f))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["Ptr", "FuncPtr"]);
}

#[test]
fn converting_to_an_integer_rounds_with_ties_to_even() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Sub Main()
    Console.WriteLine(CInt(3.9))
    Console.WriteLine(CInt(3.1))
    Console.WriteLine(CInt(-3.9))

    ' Ties go to the even neighbour, as they do in VB.NET and VBA.
    Console.WriteLine(CInt(2.5))
    Console.WriteLine(CInt(3.5))
    Console.WriteLine(CInt(-2.5))

    Console.WriteLine(CLng(1234.6))
    Console.WriteLine(CByte(200.7))
    Console.WriteLine(CInt("3.9"))

    ' An assignment converts the same way a conversion does.
    Dim narrowed As Integer = 7.8
    Console.WriteLine(narrowed)

    ' Int and Fix are not conversions and still do what they say.
    Console.WriteLine(Int(3.9))
    Console.WriteLine(Fix(-3.9))
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "4", "3", "-4", "2", "4", "-2", "1235", "201", "4", "8", "3", "-3"
        ]
    );
}

#[test]
fn a_tuple_holds_values_by_position_and_by_name() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Sub Main()
    Dim pair = (1, "two")
    Console.WriteLine(pair.Item1)
    Console.WriteLine(pair.Item2)

    ' A name is another way to reach the same element, not a different one.
    Dim point = (X := 3, Y := 4)
    Console.WriteLine(point.X)
    Console.WriteLine(point.Y)
    Console.WriteLine(point.Item1)
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "two", "3", "4", "3"]);
}

#[test]
fn a_function_can_declare_and_return_a_tuple() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Function Divide(ByVal a As Long, ByVal b As Long) As (Quotient As Long, Remainder As Long)
    Return (Quotient := a \ b, Remainder := a Mod b)
End Function

Sub Main()
    Dim result As (Quotient As Long, Remainder As Long)
    result = Divide(17, 5)
    Console.WriteLine(result.Quotient)
    Console.WriteLine(result.Remainder)

    ' Elements convert on the way in, the way any other value does.
    Dim widened As (Double, Double)
    widened = (1, 2)
    Console.WriteLine(widened.Item1)
End Sub
"#,
    );

    assert_eq!(output, vec!["3", "2", "1"]);
}

#[test]
fn dim_with_parentheses_names_a_tuples_elements() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Function Divide(ByVal a As Long, ByVal b As Long) As (Quotient As Long, Remainder As Long)
    Return (Quotient := a \ b, Remainder := a Mod b)
End Function

Sub Main()
    Dim (q, r) = Divide(17, 5)
    Console.WriteLine(q & " remainder " & r)

    Dim (a, b) = (1, "two")
    Console.WriteLine(a & "/" & b)
End Sub
"#,
    );

    assert_eq!(output, vec!["3 remainder 2", "1/two"]);
}

#[test]
fn a_tuple_reports_an_element_it_does_not_have() {
    let diagnostic = crate::backend::interpreter::tests::helpers::source_diagnostic(
        r#"
Sub Main()
    Dim point As (X As Long, Y As Long)
    Console.WriteLine(point.Z)
End Sub
"#,
    );

    assert!(diagnostic.message.contains("A tuple has no element 'Z'"));
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("X") && note.contains("Y"))
    );
}

#[test]
fn naming_the_wrong_number_of_elements_is_reported() {
    let error = crate::backend::interpreter::tests::helpers::source_error(
        r#"
Sub Main()
    Dim (a, b, c) = (1, 2)
End Sub
"#,
    );

    assert!(error.contains("3 names for a tuple of 2 elements"));
}

#[test]
fn a_parenthesised_expression_is_still_just_an_expression() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Sub Main()
    Console.WriteLine((1 + 2) * 3)
End Sub
"#,
    );

    assert_eq!(output, vec!["9"]);
}

#[test]
fn an_anonymous_type_is_a_value_with_named_members() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Sub Main()
    Dim person = New With { Key .Name = "Ada", .Age = 36 }
    Console.WriteLine(person.Name & " is " & person.Age)

    ' It behaves as any other grouped value does: assigning copies it.
    Dim copy_ = person
    Console.WriteLine(copy_.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Ada is 36", "Ada"]);
}

#[test]
fn an_anonymous_type_reports_a_member_it_does_not_have() {
    let diagnostic = crate::backend::interpreter::tests::helpers::source_diagnostic(
        r#"
Function Make() As (Name As String, Age As Long)
    Return New With { .Name = "Ada", .Age = 36 }
End Function

Sub Main()
    Console.WriteLine(Make().Height)
End Sub
"#,
    );

    assert!(diagnostic.message.contains("has no element 'Height'"));
}

#[test]
fn option_strict_rejects_conversions_that_can_lose_something() {
    let run = crate::backend::interpreter::tests::helpers::run_source;
    let error = crate::backend::interpreter::tests::helpers::source_error;

    // Widening is untouched: nothing is lost putting an Integer in a Double.
    assert_eq!(
        run(r#"
Option Strict On

Sub Main()
    Dim widened As Double = 3
    Console.WriteLine(widened)
End Sub
"#),
        vec!["3"]
    );

    let narrowing = error(
        r#"
Option Strict On

Sub Main()
    Dim narrowed As Integer = 3.9
End Sub
"#,
    );
    assert!(narrowing.contains("Option Strict does not allow Double to become Integer"));

    let parsing = error(
        r#"
Option Strict On

Sub Main()
    Dim parsed As Long = "7"
End Sub
"#,
    );
    assert!(parsing.contains("does not allow String to become Long"));

    // Saying so explicitly is always allowed.
    assert_eq!(
        run(r#"
Option Strict On

Sub Main()
    Dim rounded As Integer = CInt(3.9)
    Console.WriteLine(rounded)
End Sub
"#),
        vec!["4"]
    );
}

#[test]
fn option_strict_rejects_late_binding() {
    let error = crate::backend::interpreter::tests::helpers::source_error(
        r#"
Option Strict On

Class Thing
    Public Name_ As String
End Class

Sub Main()
    Dim thing As New Thing()
    Dim loose As Variant
    loose = thing
    Console.WriteLine(loose.Name_)
End Sub
"#,
    );

    assert!(error.contains("Option Strict does not allow reaching 'Name_' on Variant"));
}

#[test]
fn option_strict_is_off_unless_it_is_asked_for() {
    let output = crate::backend::interpreter::tests::helpers::run_source(
        r#"
Option Strict Off

Sub Main()
    Dim narrowed As Integer = 3.9
    Console.WriteLine(narrowed)
End Sub
"#,
    );

    assert_eq!(output, vec!["4"]);
}
