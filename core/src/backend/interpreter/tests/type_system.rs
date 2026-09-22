use super::helpers::{run_source, source_diagnostic, source_error};

#[test]
fn structure_implements_interface() {
    let output = run_source(
        r#"
Interface ITest
    Sub Print()
End Interface

Structure DocumentInfo
    Implements ITest

    Public Property Code As Integer

    Public Sub Print() Implements ITest.Print
        Console.WriteLine("Code: " & Me.Code)
    End Sub
End Structure

Sub Main()
    Dim doc As DocumentInfo
    doc.Code = 123
    
    doc.Print()
End Sub
"#,
    );

    assert_eq!(output, vec!["Code: 123"]);
}

#[test]
fn structure_polymorphism_through_interface() {
    let output = run_source(
        r#"
Interface ITest
    Sub Print()
End Interface

Structure DocumentInfo
    Implements ITest

    Public Property Code As Integer

    Public Sub Print() Implements ITest.Print
        Console.WriteLine("Code: " & Me.Code)
    End Sub
End Structure

Sub Main()
    Dim doc As DocumentInfo
    doc.Code = 456
    
    Dim it As ITest
    it = doc
    it.Print()
End Sub
"#,
    );

    assert_eq!(output, vec!["Code: 456"]);
}

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
    p = New Player()
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
    b = New Box()
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
    x = New Box(Of String)()
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
    x = New Box(Of String)()
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
    box = New Box(Of String)()
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
    MakeDefault = Nothing
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
    x = New Box(Of Box(Of String))()
    x.Value = New Box(Of String)()
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
    user = New User()

    Dim box As Box(Of User)
    box = New Box(Of User)()
    box.Value = user

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
    cage = New Cage(Of Dog)()
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

#[test]
fn is_nothing_tests_a_nullable_for_emptiness() {
    let output = run_source(
        r#"
Sub Main()
    Dim x As Integer? = Nothing
    Console.WriteLine(x Is Nothing)
    x = 42
    Console.WriteLine(x IsNot Nothing)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "True"]);
}

#[test]
fn ordering_comparison_accepts_variant_operands() {
    let output = run_source(
        r#"
Sub Main()
    Dim a As Variant = 3
    Dim b As Variant = 7
    Console.WriteLine(a < b)
End Sub
"#,
    );

    assert_eq!(output, vec!["True"]);
}

#[test]
fn array_field_of_a_user_type_can_be_indexed_through_a_member() {
    let output = run_source(
        r#"
Private Type BufferState
    Values(0 To 1) As Long
End Type

Sub Main()
    Dim state As BufferState
    state.Values(0) = 11
    Console.WriteLine(state.Values(0))
End Sub
"#,
    );

    assert_eq!(output, vec!["11"]);
}

#[test]
fn generic_constructor_arguments_are_checked_against_the_type_argument() {
    let output = run_source(
        r#"
Class Slot(Of T)
    Public Value As T

    Public Sub Initialize(ByVal value As T)
        Me.Value = value
    End Sub
End Class

Sub Main()
    Dim item As Slot(Of String)
    item = New Slot(Of String)("runtime")
    Console.WriteLine(item.Value)
End Sub
"#,
    );

    assert_eq!(output, vec!["runtime"]);
}

#[test]
fn a_shared_property_is_reachable_by_its_bare_name_inside_the_class() {
    let output = run_source(
        r#"
Class Counter
    Public Shared Property InstanceCount As Integer = 0

    Public Sub New()
        InstanceCount = InstanceCount + 1
    End Sub
End Class

Sub Main()
    Dim first As New Counter()
    Dim second As New Counter()
    Console.WriteLine(Counter.InstanceCount)
End Sub
"#,
    );

    assert_eq!(output, vec!["2"]);
}

#[test]
fn add_handler_rejects_an_event_the_class_does_not_declare() {
    let diagnostic = source_diagnostic(
        r#"
Class Source
    Public Event Click()
End Class

Sub OnOther()
End Sub

Sub Main()
    Dim src As New Source()
    AddHandler src.Missing, AddressOf OnOther
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::MEMBER_ACCESS
    );
    assert!(diagnostic.message.contains("does not declare event"));
}

#[test]
fn a_constrained_type_parameter_has_the_members_of_its_bound() {
    let output = run_source(
        r#"
Interface IShape
    Function Area() As Double
End Interface

Class Circle
    Implements IShape
    Public R As Double
    Public Function Area() As Double Implements IShape.Area
        Return R * R
    End Function
End Class

' The constraint is what makes shape.Area() resolve: without it, T is a name
' with nothing on it.
Function Measure(Of T As IShape)(ByVal shape As T) As Double
    Return shape.Area()
End Function

Sub Main()
    Dim c As New Circle()
    c.R = 3
    Console.WriteLine(Measure(Of Circle)(c))
End Sub
"#,
    );

    assert_eq!(output, vec!["9"]);
}

#[test]
fn implementing_an_interface_satisfies_a_bound() {
    let output = run_source(
        r#"
Interface INamed
    Function Name_() As String
End Interface

Class Tag
    Implements INamed
    Public Function Name_() As String Implements INamed.Name_
        Return "tag"
    End Function
End Class

Function Describe(Of T As INamed)(ByVal item As T) As String
    Return item.Name_()
End Function

Sub Main()
    Dim t As New Tag()
    Console.WriteLine(Describe(Of Tag)(t))
End Sub
"#,
    );

    assert_eq!(output, vec!["tag"]);
}

#[test]
fn a_new_constraint_lets_the_parameter_be_constructed() {
    let output = run_source(
        r#"
Class Counter
    Public Total As Long
End Class

Function Fresh(Of T As New)() As T
    Return New T()
End Function

Sub Main()
    Dim c As Counter
    c = Fresh(Of Counter)()
    Console.WriteLine(c.Total)
End Sub
"#,
    );

    assert_eq!(output, vec!["0"]);
}

#[test]
fn a_type_argument_that_breaks_a_constraint_is_rejected() {
    let not_a_class = source_error(
        r#"
Class Box_(Of T As Class)
    Public Item As T
End Class

Sub Main()
    Dim bad As New Box_(Of Long)()
End Sub
"#,
    );
    assert!(not_a_class.contains("must be a reference type"));

    let missing_interface = source_error(
        r#"
Interface IShape
    Function Area() As Double
End Interface

Class Plain
End Class

Function Measure(Of T As IShape)(ByVal shape As T) As Double
    Return shape.Area()
End Function

Sub Main()
    Dim p As New Plain()
    Console.WriteLine(Measure(Of Plain)(p))
End Sub
"#,
    );
    assert!(missing_interface.contains("must inherit from or implement 'IShape'"));
}

#[test]
fn a_constraint_does_not_leak_into_another_declaration() {
    let error = source_error(
        r#"
Interface IShape
    Function Area() As Double
End Interface

Function Measure(Of T As IShape)(ByVal shape As T) As Double
    Return shape.Area()
End Function

' The same letter, no constraint: Area is not available here.
Function Loose(Of T)(ByVal thing As T) As Double
    Return thing.Area()
End Function

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Class or Structure 'T' is not defined") || error.contains("'T'"));
}
