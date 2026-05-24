use super::helpers::{run_source, source_error};

#[test]
fn interface_implements_sub_contract() {
    let output = run_source(
        r#"
Interface IUpdatable
    Sub Update()
End Interface

Class Player Implements IUpdatable
    Public Sub Update() Implements IUpdatable.Update
        Debug.Print "Updating"
    End Sub
End Class

Sub Main()
    Dim p As Player
    Set p = New Player()
    p.Update()
End Sub
"#,
    );

    assert_eq!(output, vec!["Updating"]);
}

#[test]
fn interface_missing_member_is_rejected() {
    let error = source_error(
        r#"
Interface IUpdatable
    Sub Update()
End Interface

Class Player Implements IUpdatable
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("missing implementation"));
}

#[test]
fn shared_function_and_field_dispatch_through_class_name() {
    let output = run_source(
        r#"
Class MathUtil
    Public Shared PI As Double

    Public Shared Function Add(ByVal a As Double, ByVal b As Double) As Double
        Add = a + b
    End Function
End Class

Sub Main()
    Debug.Print MathUtil.Add(2#, 3#)
    Debug.Print MathUtil.PI
End Sub
"#,
    );

    assert_eq!(output, vec!["5", "0"]);
}

#[test]
fn friend_visibility_is_accepted_as_internal_visibility() {
    let output = run_source(
        r#"
Friend Class Box
    Friend Function Value() As Integer
        Value = 7
    End Function
End Class

Sub Main()
    Dim b As Box
    Set b = New Box()
    Debug.Print b.Value()
End Sub
"#,
    );

    assert_eq!(output, vec!["7"]);
}

#[test]
fn structure_sub_new_constructor_initializes_fields() {
    let output = run_source(
        r#"
Structure Vec3
    X As Double
    Y As Double
    Z As Double

    Public Sub New(ByVal x As Double, ByVal y As Double, ByVal z As Double)
        Me.X = x
        Me.Y = y
        Me.Z = z
    End Sub

    Public Function Sum() As Double
        Sum = X + Y + Z
    End Function
End Structure

Sub Main()
    Dim v As Vec3
    v = New Vec3(1#, 2#, 3#)
    Debug.Print v.Sum()
End Sub
"#,
    );

    assert_eq!(output, vec!["6"]);
}

#[test]
fn generic_class_field_uses_instantiated_type() {
    let output = run_source(
        r#"
Class Box(Of T)
    Public Value As T
End Class

Sub Main()
    Dim x As Box(Of String)
    Set x = New Box(Of String)()
    x.Value = "hello"
    Debug.Print x.Value
End Sub
"#,
    );

    assert_eq!(output, vec!["hello"]);
}

#[test]
fn generic_class_rejects_wrong_field_assignment() {
    let error = source_error(
        r#"
Class Box(Of T)
    Public Value As T
End Class

Sub Main()
    Dim x As Box(Of String)
    Set x = New Box(Of String)()
    x.Value = 123
End Sub
"#,
    );

    assert!(error.contains("Cannot assign"));
    assert!(error.contains("String"));
}

#[test]
fn generic_structure_preserves_concrete_field_types() {
    let output = run_source(
        r#"
Structure Pair(Of A, B)
    Public Left As A
    Public Right As B
End Structure

Sub Main()
    Dim p As Pair(Of String, Long)
    p.Left = "age"
    p.Right = 42
    Debug.Print p.Left
    Debug.Print p.Right
End Sub
"#,
    );

    assert_eq!(output, vec!["age", "42"]);
}

#[test]
fn generic_function_explicit_type_arguments() {
    let output = run_source(
        r#"
Function Identity(Of T)(ByVal value As T) As T
    Identity = value
End Function

Sub Main()
    Debug.Print Identity(Of String)("hello")
End Sub
"#,
    );

    assert_eq!(output, vec!["hello"]);
}

#[test]
fn nested_generic_type_names_parse_and_validate() {
    let output = run_source(
        r#"
Class Box(Of T)
    Public Value As T
End Class

Sub Main()
    Dim x As Box(Of Box(Of String))
    Set x = New Box(Of Box(Of String))()
    Set x.Value = New Box(Of String)()
    x.Value.Value = "nested"
    Debug.Print x.Value.Value
End Sub
"#,
    );

    assert_eq!(output, vec!["nested"]);
}
