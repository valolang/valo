use crate::backend::interpreter::tests::helpers::*;
use crate::frontend::parser::Parser;

#[test]
fn reports_division_by_zero() {
    let error = source_error(
        r#"
Sub Main()
    Dim x As Integer
    x = 1 / 0
End Sub
"#,
    );

    assert!(error.contains("Division by zero"));
    assert!(error.contains("line 4, column"));
}

#[test]
fn reports_undefined_variables() {
    let error = source_error(
        r#"
Option Explicit
Sub Main()
    missing = 1
End Sub
"#,
    );

    assert!(error.contains("Variable 'missing' is not declared"));
}

#[test]
fn unknown_variable_reports_nearby_symbol() {
    let diagnostic = source_diagnostic(
        r#"
Option Explicit
Sub Main()
    Dim Count As Integer
    Conut = 1
End Sub
"#,
    );

    assert!(
        diagnostic
            .message
            .contains("Variable 'Conut' is not declared")
    );
    assert!(
        diagnostic
            .helps
            .iter()
            .any(|help| help.contains("did you mean 'count'?"))
    );
}

#[test]
fn reports_type_mismatch_errors() {
    let error = source_error(
        r#"
Sub Main()
    Dim x As Integer
    x = "nope"
End Sub
"#,
    );

    assert!(error.contains("Cannot assign String value to Integer variable"));
    assert!(error.contains("line 4, column"));
}

#[test]
fn reports_unknown_function() {
    let error = source_error(
        r#"
Sub Main()
    Console.WriteLine(Missing())
End Sub
"#,
    );

    assert!(error.contains("Function 'Missing' is not defined"));
}

#[test]
fn byref_literal_argument_works_as_copy() {
    let output = run_source(
        r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
    Debug.Print "value=" & value
End Sub

Sub Main()
    Increment(10)
End Sub
"#,
    );

    assert!(output.contains(&"value=11".to_string()));
}

#[test]
fn byref_expression_argument_works_as_copy() {
    let output = run_source(
        r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
    Debug.Print "value=" & value
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x + 1)
    Debug.Print "x=" & x
End Sub
"#,
    );

    assert!(output.contains(&"value=12".to_string()));
    assert!(output.contains(&"x=10".to_string()));
}

#[test]
fn reports_wrong_sub_argument_count() {
    let error = source_error(
        r#"
Sub Show(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Main()
    Show()
End Sub
"#,
    );

    assert!(error.contains("Sub 'Show' expects 1 argument(s), got 0"));
}

#[test]
fn reports_unknown_sub() {
    let error = source_error(
        r#"
Sub Main()
    Missing()
End Sub
"#,
    );

    assert!(error.contains("Sub 'Missing' is not defined"));
}

#[test]
fn reports_duplicate_sub_name() {
    let error = source_error(
        r#"
Sub Same()
End Sub

Sub SAME()
End Sub

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Name 'SAME' conflicts with existing Sub"));
}

#[test]
fn rejects_main_with_parameters() {
    let error = source_error(
        r#"
Sub Main(ByVal value As Integer)
    Console.WriteLine(value)
End Sub
"#,
    );

    assert!(error.contains("Sub Main() cannot have parameters"));
}

#[test]
fn reports_sub_used_in_expression() {
    let error = source_error(
        r#"
Sub SayHello()
    Console.WriteLine("Hello")
End Sub

Sub Main()
    Dim value As Integer
    value = SayHello()
End Sub
"#,
    );

    assert!(error.contains("Sub 'SayHello' cannot be used as an expression"));
}

#[test]
fn reports_unknown_type() {
    let error = source_error(
        r#"
Sub Main()
    Dim user As Missing
End Sub
"#,
    );

    assert!(error.contains("Type 'Missing' is not defined"));
}

#[test]
fn reports_duplicate_type() {
    let error = source_error(
        r#"
Type User
    Name As String
End Type

Type user
    Age As Integer
End Type

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Type 'user' is already defined"));
}

#[test]
fn reports_duplicate_field() {
    let error = source_error(
        r#"
Type User
    Name As String
    NAME As String
End Type

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Field 'NAME' is already declared in Type 'User'"));
}

#[test]
fn reports_unknown_field() {
    let error = source_error(
        r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Age)
End Sub
"#,
    );

    assert!(error.contains("Type 'User' has no field 'Age'"));
}

#[test]
fn reports_field_type_mismatch() {
    let error = source_error(
        r#"
Type User
    Age As Integer
End Type

Sub Main()
    Dim user As User
    user.Age = "old"
End Sub
"#,
    );

    assert!(error.contains("Cannot assign String value to Integer variable"));
}

#[test]
fn reports_mod_by_zero() {
    let error = source_error(
        r#"
Sub Main()
    Console.WriteLine(10 Mod 0)
End Sub
"#,
    );

    assert!(error.contains("Modulo by zero"));
}

use crate::runtime::FileId;

#[test]
fn malformed_case_is_has_readable_diagnostic() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    Dim value As Integer
    Select Case value
        Case Is
            Console.WriteLine("bad")
    End Select
End Sub
"#,
        FileId::default(),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("Expected comparison operator after 'Case Is'"));
}

#[test]
fn named_argument_errors_are_clear() {
    let duplicate = source_error(
        r#"
Sub Greet(ByVal name As String)
End Sub

Sub Main()
    Greet name := "a", name := "b"
End Sub
"#,
    );
    assert!(duplicate.contains("specified more than once"));

    let unknown = source_error(
        r#"
Sub Greet(ByVal name As String)
End Sub

Sub Main()
    Greet missing := "a"
End Sub
"#,
    );
    assert!(unknown.contains("no parameter named 'missing'"));

    let positional_after_named = source_error(
        r#"
Sub Greet(ByVal first As String, ByVal second As String)
End Sub

Sub Main()
    Greet first := "a", "b"
End Sub
"#,
    );
    assert!(
        positional_after_named.contains("Positional arguments cannot appear after named arguments")
    );
}

#[test]
fn ismissing_rejects_non_optional_variable_and_missing_direct_use() {
    let non_optional = source_error(
        r#"
Sub Main()
    Dim value As Variant
    Console.WriteLine(IsMissing(value))
End Sub
"#,
    );
    assert!(non_optional.contains("IsMissing is only valid for Optional parameters"));

    let direct_use = source_error(
        r#"
Sub Greet(Optional ByVal name As Variant)
    Console.WriteLine(name)
End Sub

Sub Main()
    Greet
End Sub
"#,
    );
    assert!(direct_use.contains("Optional argument was omitted here"));
}

#[test]
fn like_rejects_non_string_operands() {
    let error = source_error(
        r#"
Sub Main()
    Console.WriteLine(10 Like "1*")
End Sub
"#,
    );

    assert!(error.contains("Cannot assign Integer value to String variable"));
}

#[test]
fn option_explicit_is_recognized() {
    let output = run_source(
        r#"
Option Explicit

Sub Main()
    Dim x As Integer
    x = 10
    Console.WriteLine(x)
End Sub
"#,
    );
    assert_eq!(output, vec!["10"]);

    let after_decl = source_error(
        r#"
Sub Main()
End Sub
Option Explicit
"#,
    );
    assert!(after_decl.contains("Option statements must appear before declarations"));

    let duplicate = source_error(
        r#"
Option Explicit
Option Explicit
Sub Main()
End Sub
"#,
    );
    assert!(duplicate.contains("Option Explicit is already declared"));
}

#[test]
fn option_base_rejects_invalid_duplicate_and_late_declarations() {
    let invalid = source_error(
        r#"
Option Base 2
Sub Main()
End Sub
"#,
    );
    assert!(invalid.contains("Option Base must be 0 or 1"));

    let duplicate = source_error(
        r#"
Option Base 0
Option Base 1
Sub Main()
End Sub
"#,
    );
    assert!(duplicate.contains("Option Base is already declared"));

    let late = source_error(
        r#"
Sub Main()
End Sub
Option Base 1
"#,
    );
    assert!(late.contains("Option statements must appear before declarations"));
}

#[test]
fn option_compare_rejects_duplicate_unknown_and_late_declarations() {
    let duplicate = source_error(
        r#"
Option Compare Binary
Option Compare Text
Sub Main()
End Sub
"#,
    );
    assert!(duplicate.contains("Option Compare is already declared"));

    let unknown = source_error(
        r#"
Option Compare Database
Sub Main()
End Sub
"#,
    );
    assert!(unknown.contains("Option Compare must be Binary or Text"));

    let late = source_error(
        r#"
Sub Main()
End Sub
Option Compare Text
"#,
    );
    assert!(late.contains("Option statements must appear before declarations"));
}

#[test]
fn conditional_compilation_reports_structure_errors() {
    let missing = source_error(
        r#"
#If True Then
Sub Main()
End Sub
"#,
    );
    assert!(missing.contains("Missing '#End If'"));

    let unexpected_else = source_error(
        r#"
#Else
Sub Main()
End Sub
"#,
    );
    assert!(unexpected_else.contains("Unexpected '#Else'"));

    let unexpected_end = source_error(
        r#"
#End If
Sub Main()
End Sub
"#,
    );
    assert!(unexpected_end.contains("Unexpected '#End If'"));
}
