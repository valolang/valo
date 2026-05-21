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
