use crate::interpreter::tests::helpers::*;

#[test]
fn vba_array_bounds_lbound_ubound_dimension_and_erase_work() {
    let output = run_source(
        r#"
Sub Main()
    Dim fixed(2 To 4) As Integer
    fixed(2) = 10
    Console.WriteLine(LBound(fixed, 1) & ":" & UBound(fixed, 1) & ":" & fixed(2))

    Dim dynamic() As Integer
    ReDim dynamic(0 To 2)
    dynamic(2) = 7
    Console.WriteLine(LBound(dynamic) & ":" & UBound(dynamic) & ":" & dynamic(2))
    Erase dynamic
    Console.WriteLine(IsArray(dynamic))
End Sub
"#,
    );

    assert_eq!(output, vec!["2:4:10", "0:2:7", "True"]);
}

#[test]
fn on_error_resume_next_suppresses_runtime_errors_and_populates_err() {
    let output = run_source(
        r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim x As Integer
    Dim values(1) As Integer
    Dim user As User

    On Error Resume Next
    x = 1 / 0
    Console.WriteLine(Err.Number > 0)
    Console.WriteLine(Err.Description)
    Err.Clear()
    Console.WriteLine(Err.Number)
    Console.WriteLine(Err.Description)

    Console.WriteLine(user.Name)
    Console.WriteLine(Err.Number > 0)

    values(2) = 10
    Console.WriteLine(Err.Description)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "True",
            "Division by zero",
            "0",
            "",
            "True",
            "Array index 2 is out of bounds for length 2",
        ]
    );
}

#[test]
fn resume_retries_original_statement_after_state_is_fixed() {
    let output = run_source(
        r#"
Sub Main()
    Dim values() As Integer
    On Error GoTo Handler
    values(0) = 7
    Console.WriteLine(values(0))
    GoTo Done
Handler:
    ReDim values(0)
    Resume
Done:
    Console.WriteLine("done")
End Sub
"#,
    );

    assert_eq!(output, vec!["7", "done"]);
}

#[test]
fn declares_fixed_integer_array() {
    let output = run_source(
        r#"
Sub Main()
    Dim numbers(3) As Integer
    Console.WriteLine(numbers(0))
    Console.WriteLine(numbers(3))
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "0"]);
}

#[test]
fn assigns_and_reads_array_elements() {
    let output = run_source(
        r#"
Sub Main()
    Dim numbers(3) As Integer
    numbers(0) = 10
    numbers(1) = 20
    numbers(2) = 30
    Console.WriteLine(numbers(0))
    Console.WriteLine(numbers(1))
    Console.WriteLine(numbers(2))
End Sub
"#,
    );

    assert_eq!(output, vec!["10", "20", "30"]);
}

#[test]
fn supports_expression_array_index() {
    let output = run_source(
        r#"
Sub Main()
    Dim numbers(3) As Integer
    Dim i As Integer
    i = 1
    numbers(i + 1) = 42
    Console.WriteLine(numbers(2))
End Sub
"#,
    );

    assert_eq!(output, vec!["42"]);
}

#[test]
fn reports_array_bounds_error() {
    let error = source_error(
        r#"
Sub Main()
    Dim numbers(1) As Integer
    Console.WriteLine(numbers(2))
End Sub
"#,
    );

    assert!(error.contains("Array index 2 is out of bounds for length 2"));
}

#[test]
fn reports_scalar_used_as_array() {
    let error = source_error(
        r#"
Sub Main()
    Dim number As Integer
    Console.WriteLine(number(0))
End Sub
"#,
    );

    assert!(error.contains("Variable 'number' is not an array"));
}

#[test]
fn reports_wrong_array_element_type() {
    let error = source_error(
        r#"
Sub Main()
    Dim numbers(1) As Integer
    numbers(0) = "nope"
End Sub
"#,
    );

    assert!(error.contains("Cannot assign String value to Integer variable"));
}

#[test]
fn supports_array_of_user_defined_type() {
    let output = run_source(
        r#"
Type User
    Name As String
    Age As Integer
End Type

Sub Main()
    Dim users(2) As User
    users(0).Name = "Valo"
    users(0).Age = 1
    Console.WriteLine(users(0).Name)
    Console.WriteLine(users(0).Age)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "1"]);
}

#[test]
fn reports_array_used_as_scalar() {
    let error = source_error(
        r#"
Sub Main()
    Dim numbers(1) As Integer
    Console.WriteLine(numbers)
End Sub
"#,
    );

    assert!(error.contains("Array variable 'numbers' cannot be used as a scalar"));
}

#[test]
fn paramarray_packs_extra_arguments_and_may_be_omitted() {
    let output = run_source(
        r#"
Sub PrintAll(ParamArray values() As Variant)
    Dim item As Variant
    Console.WriteLine("start")
    For Each item In values
        Console.WriteLine(item)
    Next item
End Sub

Sub Main()
    PrintAll
    PrintAll "a", 2, True
End Sub
"#,
    );

    assert_eq!(output, vec!["start", "start", "a", "2", "True"]);
}

#[test]
fn dynamic_arrays_redim_bounds_and_for_each_work() {
    let output = run_source(
        r#"
Sub Main()
    Dim values() As Integer
    ReDim values(2)
    values(0) = 10
    values(1) = 20
    values(2) = 30
    Console.WriteLine(LBound(values))
    Console.WriteLine(UBound(values))

    ReDim Preserve values(4)
    values(3) = 40
    values(4) = 50
    ReDim Preserve values(1)
    Console.WriteLine(UBound(values))

    Dim item As Variant
    For Each item In values
        Console.WriteLine(item)
    Next item
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "2", "1", "10", "20"]);
}

#[test]
fn redim_without_preserve_discards_contents() {
    let output = run_source(
        r#"
Sub Main()
    Dim values() As Integer
    ReDim values(1)
    values(0) = 99
    ReDim values(1)
    Console.WriteLine(values(0))
End Sub
"#,
    );

    assert_eq!(output, vec!["0"]);
}

#[test]
fn dynamic_arrays_support_class_and_type_defaults() {
    let output = run_source(
        r#"
Class User
    Public Name As String
End Class

Type Point
    X As Integer
End Type

Sub Main()
    Dim users() As User
    ReDim users(0)
    If users(0) Is Nothing Then
        Console.WriteLine("nothing")
    End If

    Dim points() As Point
    ReDim points(0)
    Console.WriteLine(points(0).X)
End Sub
"#,
    );

    assert_eq!(output, vec!["nothing", "0"]);
}

#[test]
fn dynamic_array_errors_are_clear() {
    let unallocated = source_error(
        r#"
Sub Main()
    Dim values() As Integer
    Console.WriteLine(values(0))
End Sub
"#,
    );
    assert!(unallocated.contains("Dynamic array is unallocated"));

    let negative = source_error(
        r#"
Sub Main()
    Dim values() As Integer
    ReDim values(-1)
End Sub
"#,
    );
    assert!(negative.contains("ReDim upper bound must be non-negative"));

    let fixed = source_error(
        r#"
Sub Main()
    Dim values(1) As Integer
    ReDim values(2)
End Sub
"#,
    );
    assert!(fixed.contains("ReDim target must be a dynamic array"));
}

#[test]
fn lbound_ubound_reject_unallocated_scalar_and_wrong_count() {
    let unallocated = source_error(
        r#"
Sub Main()
    Dim values() As Integer
    Console.WriteLine(UBound(values))
End Sub
"#,
    );
    assert!(unallocated.contains("Dynamic array is unallocated"));

    let scalar = source_error(
        r#"
Sub Main()
    Dim value As Integer
    Console.WriteLine(LBound(value))
End Sub
"#,
    );
    assert!(scalar.contains("Variable 'value' is not an array"));

    let wrong_count = source_error(
        r#"
Sub Main()
    Dim values(1) As Integer
    Console.WriteLine(UBound(values, 2))
End Sub
"#,
    );
    assert!(wrong_count.contains("Only one-dimensional arrays are supported"));
}

#[test]
fn for_each_supports_fixed_arrays_exit_for_nested_and_next_validation() {
    let output = run_source(
        r#"
Sub Main()
    Dim values(2) As Integer
    values(0) = 1
    values(1) = 2
    values(2) = 3

    Dim item As Integer
    For Each item In values
        If item = 3 Then
            Exit For
        End If
        Console.WriteLine(item)
    Next item

    Dim other As Integer
    For Each item In values
        For Each other In values
            If other = 2 Then
                Exit For
            End If
            Console.WriteLine(item & ":" & other)
        Next other
    Next item
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2", "1:1", "2:1", "3:1"]);

    let mismatch = source_error(
        r#"
Sub Main()
    Dim values(1) As Integer
    Dim item As Integer
    For Each item In values
    Next other
End Sub
"#,
    );
    assert!(mismatch.contains("Next variable 'other' does not match For Each variable 'item'"));
}

#[test]
fn option_base_controls_fixed_arrays_redim_lbound_ubound_and_for_each() {
    let default_output = run_source(
        r#"
Sub Main()
    Dim a(3) As Integer
    a(0) = 5
    a(3) = 8
    Console.WriteLine(LBound(a) & ":" & UBound(a) & ":" & a(0) & ":" & a(3))
End Sub
"#,
    );
    assert_eq!(default_output, vec!["0:3:5:8"]);

    let fixed_output = run_source(
        r#"
Option Base 1
Sub Main()
    Dim a(3) As Integer
    Dim item As Integer
    Dim total As Integer
    a(1) = 10
    a(3) = 30
    For Each item In a
        total = total + item
    Next item
    Console.WriteLine(LBound(a) & ":" & UBound(a) & ":" & total)
End Sub
"#,
    );
    assert_eq!(fixed_output, vec!["1:3:40"]);

    let redim_output = run_source(
        r#"
Option Base 1
Sub Main()
    Dim a() As Integer
    ReDim a(2)
    a(1) = 7
    a(2) = 9
    Console.WriteLine(LBound(a) & ":" & UBound(a) & ":" & a(1) & ":" & a(2))
End Sub
"#,
    );
    assert_eq!(redim_output, vec!["1:2:7:9"]);
}

#[test]
fn module_level_state_defaults_persists_and_rejects_conflicts() {
    let output = run_source(
        r#"
Private counter As Integer
Public title As String
Private enabled As Boolean
Private values() As Integer
Private currentUser As User
Const Limit As Integer = 2

Class User
End Class

Sub Increment()
    counter = counter + 1
End Sub

Function NextValue() As Integer
    counter = counter + 1
    Return counter
End Function

Sub Main()
    Console.WriteLine(counter)
    Console.WriteLine("title:" & title)
    Console.WriteLine(enabled)
    If currentUser Is Nothing Then
        Console.WriteLine("nothing")
    End If
    Call Increment()
    Call Increment()
    Console.WriteLine(NextValue())
    ReDim values(Limit)
    values(2) = counter
    Console.WriteLine(values(2))
End Sub
"#,
    );
    assert_eq!(output, vec!["0", "title:", "False", "nothing", "3", "3"]);

    let const_assign = source_error(
        r#"
Const Limit As Integer = 2
Sub Main()
    Limit = 3
End Sub
"#,
    );
    assert!(const_assign.contains("Constant 'Limit' cannot be assigned"));

    let duplicate = source_error(
        r#"
Private counter As Integer
Private Counter As Integer
Sub Main()
End Sub
"#,
    );
    assert!(duplicate.contains("conflicts with existing"));

    let type_conflict = source_error(
        r#"
Type Point
    X As Integer
End Type
Private Point As Integer
Sub Main()
End Sub
"#,
    );
    assert!(type_conflict.contains("conflicts with existing Type"));
}
