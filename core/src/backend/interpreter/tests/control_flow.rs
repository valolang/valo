use crate::backend::interpreter::tests::helpers::*;
use crate::frontend::parser::Parser;

#[test]
fn runs_control_flow() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    Dim total As Integer
    i = 1
    total = 0
    While i <= 5
        total = total + i
        i = i + 1
    Wend
    If total = 15 Then
        Console.WriteLine("ok")
    Else
        Console.WriteLine("bad")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["ok"]);
}

#[test]
fn reports_line_and_column_for_parse_errors() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    Dim x As Integer
    x = 
End Sub
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("line 4, column"));
}

#[test]
fn runs_nested_if_and_while_blocks() {
    let output = run_source(
        r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 0

    While outer < 2
        inner = 0
        While inner < 2
            If outer = 1 Then
                Console.WriteLine("outer " & outer & ", inner " & inner)
            Else
                Console.WriteLine("skip")
            End If
            inner = inner + 1
        Wend
        outer = outer + 1
    Wend
End Sub
"#,
    );

    assert_eq!(
        output,
        vec!["skip", "skip", "outer 1, inner 0", "outer 1, inner 1"]
    );
}

#[test]
fn formatted_diagnostics_include_code_source_label_and_help() {
    let source = r#"
Sub Main()
    Dim age As Integer
    age = "Valo"
End Sub
"#;
    let diagnostic = source_diagnostic(source);
    let rendered = diagnostic.render("examples/test.valo", source);

    assert!(rendered.contains("error[V1100]"));
    assert!(rendered.contains("--> examples/test.valo:4:"));
    assert!(rendered.contains("expected Integer, found String"));
    assert!(rendered.contains("help: change the variable type"));
}

#[test]
fn labels_and_goto_execute_case_insensitively() {
    let output = run_source(
        r#"
Sub Main()
    GoTo done
    Console.WriteLine("skip")
Done:
    Console.WriteLine("done")
    GoTo LAST
    Console.WriteLine("skip2")
last:
    Console.WriteLine("last")
End Sub
"#,
    );

    assert_eq!(output, vec!["done", "last"]);
}

#[test]
fn labels_reject_duplicate_and_unknown_goto() {
    let duplicate = source_error(
        r#"
Sub Main()
Start:
start:
    Console.WriteLine("bad")
End Sub
"#,
    );
    assert!(duplicate.contains("Label 'start' is already declared"));

    let unknown = source_error(
        r#"
Sub Main()
    GoTo Missing
End Sub
"#,
    );
    assert!(unknown.contains("Label 'Missing' is not declared"));
}

#[test]
fn labels_do_not_break_select_case_colon_syntax() {
    let output = run_source(
        r#"
Sub Main()
    Select Case 2
    Case 1: Console.WriteLine("one")
    Case 2: Console.WriteLine("two")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["two"]);
}

#[test]
fn on_error_does_not_suppress_parse_or_semantic_errors() {
    let parse = source_error(
        r#"
Sub Main()
    On Error Resume Next
    Dim x As Integer
    x =
End Sub
"#,
    );
    assert!(parse.contains("Expected expression"));

    let semantic = source_error(
        r#"
Sub Main()
    On Error Resume Next
    GoTo Missing
End Sub
"#,
    );
    assert!(semantic.contains("Label 'Missing' is not declared"));
}

#[test]
fn on_error_goto_label_jumps_to_handler_and_exposes_err() {
    let output = run_source(
        r#"
Sub Main()
    Dim x As Integer
    On Error GoTo Handler
    x = 1 / 0
    Console.WriteLine("not reached")
Handler:
    Console.WriteLine(Err.Number > 0)
    Console.WriteLine(Err.Description)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "Division by zero"]);
}

#[test]
fn on_error_goto_zero_disables_label_handler() {
    let error = source_error(
        r#"
Sub Main()
    Dim x As Integer
    On Error GoTo Handler
    On Error GoTo 0
    x = 1 / 0
    Console.WriteLine("after")
Handler:
    Console.WriteLine("handled")
End Sub
"#,
    );

    assert!(error.contains("Division by zero"));
}

#[test]
fn err_raise_basic_and_optional_properties_are_exposed() {
    let output = run_source(
        r#"
Sub Main()
    On Error Resume Next
    Err.Raise(100)
    Console.WriteLine(Err.Number)
    Console.WriteLine(Err.Description)
    Console.WriteLine(Err.Source)
    Console.WriteLine(Err.HelpFile)
    Console.WriteLine(Err.HelpContext)
    Err.Clear()
    Console.WriteLine(Err.Number)
    Console.WriteLine(Err.Description)
    Console.WriteLine(Err.Source)
    Console.WriteLine(Err.HelpFile)
    Console.WriteLine(Err.HelpContext)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "100",
            "Application-defined or object-defined error",
            "",
            "",
            "0",
            "0",
            "",
            "",
            "",
            "0",
        ]
    );
}

#[test]
fn err_raise_populates_source_description_help_file_and_help_context() {
    let output = run_source(
        r#"
Sub Main()
    On Error Resume Next
    Err.Raise(513, "Unit.Test", "custom failure", "help.chm", 42)
    Console.WriteLine(Err.Number)
    Console.WriteLine(Err.Source)
    Console.WriteLine(Err.Description)
    Console.WriteLine(Err.HelpFile)
    Console.WriteLine(Err.HelpContext)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec!["513", "Unit.Test", "custom failure", "help.chm", "42"]
    );
}

#[test]
fn on_error_goto_minus_one_clears_active_handled_error_but_keeps_err_values() {
    let output = run_source(
        r#"
Sub Main()
    On Error GoTo Handler
    Err.Raise(1, "first", "first")
    Console.WriteLine("skip")
    GoTo Done
Handler:
    Console.WriteLine(Err.Number)
    On Error GoTo -1
    Console.WriteLine(Err.Number)
    GoTo Done
Done:
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "1"]);
}

#[test]
fn duplicate_numeric_labels_are_rejected() {
    let error = source_error(
        r#"
Sub Main()
10 Console.WriteLine("first")
10 Console.WriteLine("second")
End Sub
"#,
    );

    assert!(error.contains("Label '10' is already declared"));
}

#[test]
fn erl_returns_numeric_line_for_handled_error_and_zero_without_one() {
    let output = run_source(
        r#"
Sub Main()
    On Error GoTo Handler
10 Err.Raise(10)
    GoTo Done
Handler:
    Console.WriteLine(Erl)
    On Error GoTo -1
    Err.Clear()
    On Error GoTo OtherHandler
    Err.Raise(11)
    GoTo Done
OtherHandler:
    Console.WriteLine(Erl)
Done:
End Sub
"#,
    );

    assert_eq!(output, vec!["10", "0"]);
}

#[test]
fn runs_simple_ascending_for_loop() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2", "3"]);
}

#[test]
fn runs_descending_for_loop_with_negative_step() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 3 To 1 Step -1
        Console.WriteLine(i)
    Next
End Sub
"#,
    );

    assert_eq!(output, vec!["3", "2", "1"]);
}

#[test]
fn reports_undeclared_for_loop_variable() {
    let error = source_error(
        r#"
Sub Main()
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
    );

    assert!(error.contains("Variable 'i' is not declared"));
}

#[test]
fn reports_non_integer_for_loop_variable() {
    let error = source_error(
        r#"
Sub Main()
    Dim i As String
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
    );

    assert!(error.contains("For loop variable 'i' must be Integer"));
}

#[test]
fn runs_simple_integer_function() {
    let output = run_source(
        r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(10, 20))
End Sub
"#,
    );

    assert_eq!(output, vec!["30"]);
}

#[test]
fn runs_string_returning_function() {
    let output = run_source(
        r#"
Function Greeting(ByVal name As String) As String
    Return "Hello, " & name
End Function

Sub Main()
    Console.WriteLine(Greeting("Valo"))
End Sub
"#,
    );

    assert_eq!(output, vec!["Hello, Valo"]);
}

#[test]
fn uses_function_call_inside_expression() {
    let output = run_source(
        r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(1, 2) + Add(3, 4))
End Sub
"#,
    );

    assert_eq!(output, vec!["10"]);
}

#[test]
fn reports_wrong_function_argument_count() {
    let error = source_error(
        r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(1))
End Sub
"#,
    );

    assert!(error.contains("Function 'Add' expects 2 argument(s), got 1"));
}

#[test]
fn reports_duplicate_parameter() {
    let error = source_error(
        r#"
Function Bad(ByVal value As Integer, ByVal VALUE As Integer) As Integer
    Return value
End Function

Sub Main()
    Console.WriteLine(Bad(1, 2))
End Sub
"#,
    );

    assert!(error.contains("Parameter 'VALUE' is already declared"));
}

#[test]
fn reports_return_outside_function() {
    let error = source_error(
        r#"
Sub Main()
    Return 1
End Sub
"#,
    );

    assert!(error.contains("Return is only allowed inside Function"));
}

#[test]
fn reports_missing_return() {
    let error = source_error(
        r#"
Function MissingReturn() As Integer
    Dim x As Integer
    x = 1
End Function

Sub Main()
    Console.WriteLine(MissingReturn())
End Sub
"#,
    );

    assert!(error.contains("Function 'MissingReturn' must return a value"));
}

#[test]
fn reports_type_mismatch_return() {
    let error = source_error(
        r#"
Function Bad() As Integer
    Return "nope"
End Function

Sub Main()
    Console.WriteLine(Bad())
End Sub
"#,
    );

    assert!(error.contains("Cannot assign String value to Integer variable"));
}

#[test]
fn isolates_main_and_function_local_variables() {
    let output = run_source(
        r#"
Function GetValue() As Integer
    Dim value As Integer
    value = 99
    Return value
End Function

Sub Main()
    Dim value As Integer
    value = 1
    Console.WriteLine(GetValue())
    Console.WriteLine(value)
End Sub
"#,
    );

    assert_eq!(output, vec!["99", "1"]);
}

#[test]
fn reports_function_called_as_statement() {
    let error = source_error(
        r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Add(1, 2)
End Sub
"#,
    );

    assert!(error.contains("Function 'Add' cannot be called as a statement"));
}

#[test]
fn sub_calls_can_be_nested_and_call_functions() {
    let output = run_source(
        r#"
Function Label(ByVal value As Integer) As String
    Return "Value: " & value
End Function

Sub PrintLabel(ByVal value As Integer)
    Console.WriteLine(Label(value))
End Sub

Sub Outer(ByRef value As Integer)
    value = value + 1
    PrintLabel(value)
End Sub

Sub Main()
    Dim x As Integer
    x = 4
    Outer(x)
    Console.WriteLine(x)
End Sub
"#,
    );

    assert_eq!(output, vec!["Value: 5", "5"]);
}

#[test]
fn returns_user_defined_type_from_function() {
    let output = run_source(
        r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Function CreateUser(ByVal name As String, ByVal age As Integer) As User
    Dim u As User
    u.Name = name
    u.Age = age
    u.Active = True
    Return u
End Function

Sub Main()
    Dim user As User
    user = CreateUser("Valo", 1)
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "1", "True"]);
}

#[test]
fn returns_structure_from_function() {
    let output = run_source(
        r#"
Structure Point
    X As Integer
    Y As Integer
End Structure

Function MakePoint(ByVal x As Integer, ByVal y As Integer) As Point
    Dim p As Point
    p.X = x
    p.Y = y
    Return p
End Function

Sub Main()
    Dim p As Point
    p = MakePoint(3, 4)
    Console.WriteLine(p.X)
    Console.WriteLine(p.Y)
End Sub
"#,
    );

    assert_eq!(output, vec!["3", "4"]);
}

#[test]
fn elseif_uses_first_matching_branch() {
    let output = run_source(
        r#"
Sub Main()
    Dim age As Integer
    Dim active As Boolean
    age = 20
    active = False

    If age < 18 Then
        Console.WriteLine("Denied")
    ElseIf age >= 18 And active Then
        Console.WriteLine("Allowed")
    ElseIf age >= 18 Then
        Console.WriteLine("Inactive")
    Else
        Console.WriteLine("Other")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["Inactive"]);
}

#[test]
fn elseif_falls_through_to_else() {
    let output = run_source(
        r#"
Sub Main()
    Dim age As Integer
    age = 12

    If age > 20 Then
        Console.WriteLine("adult")
    ElseIf age = 18 Then
        Console.WriteLine("exact")
    Else
        Console.WriteLine("minor")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["minor"]);
}

#[test]
fn existing_if_without_elseif_still_works() {
    let output = run_source(
        r#"
Sub Main()
    If True Then
        Console.WriteLine("ok")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["ok"]);
}

#[test]
fn select_case_matches_integer_case() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 2

    Select Case value
        Case 1
            Console.WriteLine("one")
        Case 2
            Console.WriteLine("two")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["two"]);
}

#[test]
fn select_case_matches_string_case() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As String
    value = "b"

    Select Case value
        Case "a"
            Console.WriteLine("a")
        Case "b"
            Console.WriteLine("b")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["b"]);
}

#[test]
fn select_case_supports_multiple_values() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 4

    Select Case value
        Case 1, 2
            Console.WriteLine("low")
        Case 3, 4
            Console.WriteLine("high")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["high"]);
}

#[test]
fn next_without_variable_still_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2", "3"]);
}

#[test]
fn next_with_variable_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next i
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2", "3"]);
}

#[test]
fn next_variable_is_case_insensitive() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 2
        Console.WriteLine(i)
    Next I
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2"]);
}

#[test]
fn nested_next_variables_match_nearest_loop() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    Dim j As Integer
    For i = 1 To 2
        For j = 1 To 2
            Console.WriteLine(i & "," & j)
        Next j
    Next i
End Sub
"#,
    );

    assert_eq!(output, vec!["1,1", "1,2", "2,1", "2,2"]);
}

#[test]
fn mismatched_next_variable_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Dim i As Integer
    Dim j As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next j
End Sub
"#,
    );

    assert!(error.contains("Next variable 'j' does not match For variable 'i'"));
}

#[test]
fn select_case_integer_range_matches() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 4
    Select Case value
        Case 1 To 5
            Console.WriteLine("small")
        Case Else
            Console.WriteLine("other")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["small"]);
}

#[test]
fn select_case_integer_range_falls_through() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 8
    Select Case value
        Case 1 To 5
            Console.WriteLine("small")
        Case Else
            Console.WriteLine("other")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["other"]);
}

#[test]
fn select_case_string_range_matches() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As String
    value = "m"
    Select Case value
        Case "a" To "z"
            Console.WriteLine("letter")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["letter"]);
}

#[test]
fn select_case_mixes_values_and_ranges() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 7
    Select Case value
        Case 1, 5 To 8
            Console.WriteLine("match")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["match"]);
}

#[test]
fn select_case_is_comparisons_work() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 11
    Select Case value
        Case Is < 0
            Console.WriteLine("negative")
        Case Is >= 10
            Console.WriteLine("large")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["large"]);
}

#[test]
fn select_case_is_all_operators_work() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 5
    Select Case value
        Case Is > 10
            Console.WriteLine("gt")
        Case Is <= 4
            Console.WriteLine("lte")
        Case Is <> 5
            Console.WriteLine("ne")
        Case Is = 5
            Console.WriteLine("eq")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["eq"]);
}

#[test]
fn select_case_is_with_strings_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As String
    value = "m"
    Select Case value
        Case Is > "z"
            Console.WriteLine("after")
        Case Is <= "m"
            Console.WriteLine("up to m")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["up to m"]);
}

#[test]
fn colon_statement_separator_preserves_labels_and_select_case_colons() {
    let output = run_source(
        r#"
Sub Main()
    Dim x As Integer: x = 2: GoTo AfterSkip
SkipMe: Console.WriteLine("skip")
AfterSkip:
    Select Case x
    Case 1: Console.WriteLine("one")
    Case 2: Console.WriteLine("two")
    End Select
10 Console.WriteLine("line label"): GoTo 30
20 Console.WriteLine("skip numeric")
30 Console.WriteLine("done")
End Sub
"#,
    );

    assert_eq!(output, vec!["two", "line label", "done"]);
}

#[test]
fn optional_parameters_defaults_and_ordering_are_checked() {
    let output = run_source(
        r#"
Sub Greet(Optional ByVal name As String = "Valo")
    Console.WriteLine(name)
End Sub

Function Add(Optional ByVal a As Integer = 1, Optional ByVal b As Integer = 2) As Integer
    Return a + b
End Function

Sub Main()
    Greet
    Greet "Ada"
    Console.WriteLine(Add())
    Console.WriteLine(Add(10))
    Console.WriteLine(Add(10, 20))
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "Ada", "3", "12", "30"]);

    let error = source_error(
        r#"
Sub Bad(Optional ByVal a As Integer = 1, ByVal b As Integer)
End Sub

Sub Main()
End Sub
"#,
    );
    assert!(error.contains("Optional parameters must come after required parameters"));
}

#[test]
fn select_case_else_fallback() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 9

    Select Case value
        Case 1
            Console.WriteLine("one")
        Case Else
            Console.WriteLine("other")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["other"]);
}

#[test]
fn select_case_no_match_without_else_does_nothing() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 9

    Select Case value
        Case 1
            Console.WriteLine("one")
    End Select
    Console.WriteLine("done")
End Sub
"#,
    );

    assert_eq!(output, vec!["done"]);
}

#[test]
fn select_case_else_not_last_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Dim value As Integer
    value = 1

    Select Case value
        Case Else
            Console.WriteLine("other")
        Case 1
            Console.WriteLine("one")
    End Select
End Sub
"#,
    );

    assert!(error.contains("Case Else must be last"));
}

#[test]
fn nested_select_case_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 1
    inner = 2

    Select Case outer
        Case 1
            Select Case inner
                Case 2
                    Console.WriteLine("nested")
            End Select
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["nested"]);
}

#[test]
fn return_inside_select_case_inside_function_works() {
    let output = run_source(
        r#"
Function Label(ByVal value As Integer) As String
    Select Case value
        Case 1
            Return "one"
        Case Else
            Return "other"
    End Select
End Function

Sub Main()
    Console.WriteLine(Label(1))
End Sub
"#,
    );

    assert_eq!(output, vec!["one"]);
}

#[test]
fn do_while_runs() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do While i < 3
        Console.WriteLine(i)
        i = i + 1
    Loop
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn do_until_runs() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do Until i = 3
        Console.WriteLine(i)
        i = i + 1
    Loop
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn loop_while_runs_body_before_condition() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do
        Console.WriteLine(i)
        i = i + 1
    Loop While i < 3
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn loop_until_runs_body_before_condition() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do
        Console.WriteLine(i)
        i = i + 1
    Loop Until i = 3
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn do_loop_with_exit_do_breaks() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do
        If i = 3 Then
            Exit Do
        End If
        Console.WriteLine(i)
        i = i + 1
    Loop
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn nested_do_loops_exit_nearest_loop() {
    let output = run_source(
        r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 0
    Do While outer < 2
        inner = 0
        Do
            Exit Do
            Console.WriteLine("inner")
        Loop
        Console.WriteLine(outer)
        outer = outer + 1
    Loop
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "1"]);
}

#[test]
fn missing_loop_reports_readable_error() {
    let error = Parser::parse_source(
        r#"
Sub Main()
    Do While True
        Console.WriteLine("open")
End Sub
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("Expected 'Loop'"));
}

#[test]
fn do_loop_condition_type_mismatch_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Dim i As Integer
    i = 1
    Do While i
        Exit Do
    Loop
End Sub
"#,
    );

    assert!(error.contains("Cannot assign Integer value to Boolean variable"));
}

#[test]
fn exit_sub_skips_remaining_statements() {
    let output = run_source(
        r#"
Sub StopEarly()
    Console.WriteLine("before")
    Exit Sub
    Console.WriteLine("after")
End Sub

Sub Main()
    StopEarly()
End Sub
"#,
    );

    assert_eq!(output, vec!["before"]);
}

#[test]
fn exit_for_breaks_loop() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 5
        If i = 3 Then
            Exit For
        End If
        Console.WriteLine(i)
    Next
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2"]);
}

#[test]
fn exit_while_breaks_loop() {
    let output = run_source(
        r#"
Sub Main()
    Dim i As Integer
    i = 1
    While i <= 5
        If i = 3 Then
            Exit While
        End If
        Console.WriteLine(i)
        i = i + 1
    Wend
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2"]);
}

#[test]
fn exit_inside_select_case_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 1
    Select Case value
        Case 1
            Exit Sub
    End Select
    Console.WriteLine("after")
End Sub
"#,
    );

    assert_eq!(output, Vec::<String>::new());
}

#[test]
fn exit_for_outside_for_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Exit For
End Sub
"#,
    );

    assert!(error.contains("Exit For is only valid inside For"));
}

#[test]
fn exit_while_outside_while_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Exit While
End Sub
"#,
    );

    assert!(error.contains("Exit While is only valid inside While"));
}

#[test]
fn exit_do_outside_do_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Exit Do
End Sub
"#,
    );

    assert!(error.contains("Exit Do is only valid inside Do"));
}

#[test]
fn exit_sub_outside_sub_is_rejected() {
    let error = source_error(
        r#"
Function Value() As Integer
    Exit Sub
    Return 1
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
    );

    assert!(error.contains("Exit Sub is only valid inside Sub"));
}

#[test]
fn exit_function_in_sub_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Exit Function
End Sub
"#,
    );

    assert!(error.contains("Exit Function is only valid inside Function"));
}

#[test]
fn exit_function_in_integer_function_returns_zero() {
    let output = run_source(
        r#"
Function Value() As Integer
    Exit Function
    Return 1
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
    );

    assert_eq!(output, vec!["0"]);
}

#[test]
fn exit_function_in_string_function_returns_empty_string() {
    let output = run_source(
        r#"
Function Value() As String
    Exit Function
    Return "after"
End Function

Sub Main()
    Console.WriteLine("value:" & Value())
End Sub
"#,
    );

    assert_eq!(output, vec!["value:"]);
}

#[test]
fn exit_function_in_boolean_function_returns_false() {
    let output = run_source(
        r#"
Function Value() As Boolean
    Exit Function
    Return True
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
    );

    assert_eq!(output, vec!["False"]);
}

#[test]
fn return_expression_overrides_exit_function_default() {
    let output = run_source(
        r#"
Function Value() As Integer
    Return 42
    Exit Function
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
    );

    assert_eq!(output, vec!["42"]);
}

#[test]
fn nested_loops_exit_nearest_matching_loop() {
    let output = run_source(
        r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    For outer = 1 To 2
        For inner = 1 To 3
            If inner = 2 Then
                Exit For
            End If
            Console.WriteLine(outer & ":" & inner)
        Next
    Next
End Sub
"#,
    );

    assert_eq!(output, vec!["1:1", "2:1"]);
}

#[test]
fn enum_values_support_implicit_explicit_qualified_and_select_case() {
    let output = run_source(
        r#"
Public Enum DaysOfWeek
    Monday
    Tuesday
    Wednesday
End Enum

Public Enum FilePermissions
    Read = 1
    Write = 2
    Execute = 4
    All = Read + Write + Execute
End Enum

Public Enum WindowState
    [_First] = 1
    Normal = 1
    Minimized = 2
    Maximized = 3
    [_Last] = 3
End Enum

Sub Main()
    Dim day As DaysOfWeek
    day = Wednesday
    Select Case day
        Case Monday
            Console.WriteLine("Monday")
        Case Wednesday
            Console.WriteLine("Wednesday")
    End Select

    Dim access As FilePermissions
    access = All
    If (access And Write) = Write Then
        Console.WriteLine("Write access")
    End If

    Dim i As Integer
    For i = WindowState.[_First] To WindowState.[_Last]
        Console.WriteLine(i)
    Next i
End Sub
"#,
    );

    assert_eq!(output, vec!["Wednesday", "Write access", "1", "2", "3"]);
}

#[test]
fn const_declarations_are_immutable_and_work_in_expressions() {
    let output = run_source(
        r#"
Public Const AppName As String = "Valo"
Private Const MaxRetries As Integer = 3
Const DebugMode As Boolean = True

Sub Main()
    Const Local As Integer = MaxRetries + 2
    Console.WriteLine(AppName & " " & Local)
    If DebugMode Then
        Console.WriteLine("debug")
    End If
    Dim i As Integer
    For i = 1 To MaxRetries
    Next i
    Select Case Local
        Case 5
            Console.WriteLine("five")
    End Select
End Sub
"#,
    );
    assert_eq!(output, vec!["Valo 5", "debug", "five"]);

    let assign_error = source_error(
        r#"
Const MaxRetries As Integer = 3
Sub Main()
    MaxRetries = 4
End Sub
"#,
    );
    assert!(assign_error.contains("Constant 'MaxRetries' cannot be assigned"));

    let duplicate_error = source_error(
        r#"
Const Name As String = "a"
Const name As String = "b"
Sub Main()
End Sub
"#,
    );
    assert!(duplicate_error.contains("conflicts with existing"));

    let mismatch_error = source_error(
        r#"
Const Count As Integer = "bad"
Sub Main()
End Sub
"#,
    );
    assert!(mismatch_error.contains("Cannot assign String value to Integer variable"));

    let non_const_error = source_error(
        r#"
Function Value() As Integer
    Return 1
End Function

Const Count As Integer = Value()
Sub Main()
End Sub
"#,
    );
    assert!(non_const_error.contains("compile-time constant"));
}

#[test]
fn option_compare_controls_string_comparisons_and_select_case() {
    let binary = run_source(
        r#"
Sub Main()
    Console.WriteLine("a" = "A")
    Console.WriteLine("a" > "A")
End Sub
"#,
    );
    assert_eq!(binary, vec!["False", "True"]);

    let text = run_source(
        r#"
Option Compare Text
Sub Main()
    Console.WriteLine("a" = "A")
    Console.WriteLine("b" > "A")
    Select Case "B"
    Case "a" To "c"
        Console.WriteLine("range")
    Case Else
        Console.WriteLine("else")
    End Select
    Select Case "alpha"
    Case Is = "ALPHA"
        Console.WriteLine("is")
    End Select
End Sub
"#,
    );
    assert_eq!(text, vec!["True", "True", "range", "is"]);
}
