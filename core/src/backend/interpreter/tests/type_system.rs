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
fn generic_function_type_arguments_are_inferred_from_literals_and_variables() {
    let output = run_source(
        r#"
Function Identity(Of T)(ByVal value As T) As T
    Identity = value
End Function

Sub Main()
    Dim name As String
    name = "Valo"
    Debug.Print Identity("hello")
    Debug.Print Identity(name)
    Debug.Print Identity(42)
End Sub
"#,
    );

    assert_eq!(output, vec!["hello", "Valo", "42"]);
}

#[test]
fn generic_function_type_inference_uses_named_arguments() {
    let output = run_source(
        r#"
Function Echo(Of T)(ByVal value As T) As T
    Echo = value
End Function

Sub Main()
    Debug.Print Echo(value := "named")
End Sub
"#,
    );

    assert_eq!(output, vec!["named"]);
}

#[test]
fn generic_function_type_inference_uses_nested_generic_arguments() {
    let output = run_source(
        r#"
Class Box(Of T)
    Public Value As T
End Class

Function Unbox(Of T)(ByVal box As Box(Of T)) As T
    Unbox = box.Value
End Function

Sub Main()
    Dim box As Box(Of String)
    Set box = New Box(Of String)()
    box.Value = "nested"
    Debug.Print Unbox(box)
End Sub
"#,
    );

    assert_eq!(output, vec!["nested"]);
}

#[test]
fn generic_function_type_inference_reports_uninferrable_type_parameter() {
    let error = source_error(
        r#"
Function MakeDefault(Of T)() As T
End Function

Sub Main()
    Debug.Print MakeDefault()
End Sub
"#,
    );

    assert!(error.contains("Cannot infer type argument"));
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

#[test]
fn vbnet_style_generic_variance_and_constraints_parse() {
    let output = run_source(
        r#"
Interface IProducer(Of Out T)
    Function Current() As T
End Interface

Interface IConsumer(Of In T)
    Sub Accept(ByVal value As T)
End Interface

Class User
End Class

Class Box(Of T As {Class, New})
    Public Value As T
End Class

Function Marker(Of T)() As String Where T : Class, New
    Marker = "ok"
End Function

Sub Main()
    Dim user As User
    Set user = New User()

    Dim box As Box(Of User)
    Set box = New Box(Of User)()
    Set box.Value = user

    If box.Value Is user Then
        Debug.Print Marker(Of User)()
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["ok"]);
}

#[test]
fn generic_class_constraint_rejects_value_type_arguments() {
    let error = source_error(
        r#"
Class Box(Of T As Class)
End Class

Sub Main()
    Dim box As Box(Of Long)
End Sub
"#,
    );

    assert!(error.contains("must be a reference type"));
}

#[test]
fn generic_structure_constraint_rejects_reference_type_arguments() {
    let error = source_error(
        r#"
Structure Pair(Of T As Structure)
    Public Value As T
End Structure

Class User
End Class

Sub Main()
    Dim pair As Pair(Of User)
End Sub
"#,
    );

    assert!(error.contains("must be a value type"));
}

#[test]
fn generic_new_constraint_requires_public_parameterless_constructor() {
    let ok = run_source(
        r#"
Class User
End Class

Function Marker(Of T)() As String Where T : Class, New
    Marker = "ok"
End Function

Sub Main()
    Debug.Print Marker(Of User)()
End Sub
"#,
    );
    assert_eq!(ok, vec!["ok"]);

    let error = source_error(
        r#"
Class User
    Public Sub New(ByVal name As String)
    End Sub
End Class

Function Marker(Of T)() As String Where T : Class, New
    Marker = "ok"
End Function

Sub Main()
    Debug.Print Marker(Of User)()
End Sub
"#,
    );

    assert!(error.contains("must have a public parameterless constructor"));
}

#[test]
fn generic_base_class_constraint_allows_derived_arguments() {
    let output = run_source(
        r#"
Class Animal
End Class

Class Dog Inherits Animal
End Class

Class Cage(Of T As Animal)
    Public Occupant As T
End Class

Sub Main()
    Dim cage As Cage(Of Dog)
    Set cage = New Cage(Of Dog)()
    Debug.Print "ok"
End Sub
"#,
    );

    assert_eq!(output, vec!["ok"]);
}

#[test]
fn module_block_members_execute_as_module_level_declarations() {
    let output = run_source(
        r#"
Module MathTools
    Public Const Answer As Integer = 42

    Public Function Add(ByVal left As Integer, ByVal right As Integer) As Integer
        Add = left + right
    End Function
End Module

Sub Main()
    Debug.Print MathTools.Add(Answer, 8)
End Sub
"#,
    );

    assert_eq!(output, vec!["50"]);
}
