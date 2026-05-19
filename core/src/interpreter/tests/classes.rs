use crate::interpreter::tests::helpers::*;

#[test]
fn exported_class_attributes_default_member_as_new_and_class_initialize_work() {
    let output = run_source(
        r#"
Attribute VB_Name = "Box"

Class Box
    Private stored As Integer

    Private Sub Class_Initialize()
        stored = 11
    End Sub

    Public Property Get Value() As Integer
        Value = stored
    End Property
    Attribute Value.VB_UserMemId = 0
End Class

Function MakeBox() As Object
    Set MakeBox = New Box()
End Function

Sub Main()
    Dim a As New Box
    Console.WriteLine(a)
    Console.WriteLine(IsObject(a))
    Dim b As Object
    Set b = MakeBox()
    Console.WriteLine(TypeName(b))
End Sub
"#,
    );

    assert_eq!(output, vec!["11", "True", "Box"]);
}

#[test]
fn runtime_diagnostics_include_stack_context_when_available() {
    let source = r#"
Sub Boom()
    Dim values(1) As Integer
    values(2) = 10
End Sub

Sub Main()
    Call Boom()
End Sub
"#;
    let diagnostic = source_diagnostic(source);
    let rendered = diagnostic.render("stack.valo", source);

    assert!(rendered.contains("error[V1200]"));
    assert!(rendered.contains("note: while executing Sub 'Boom'"));
}

#[test]
fn on_error_goto_zero_disables_runtime_suppression() {
    let error = source_error(
        r#"
Sub Main()
    Dim x As Integer
    On Error Resume Next
    x = 1 / 0
    On Error GoTo 0
    x = 1 / 0
    Console.WriteLine("after")
End Sub
"#,
    );

    assert!(error.contains("Division by zero"));
}

#[test]
fn resume_next_continues_after_original_failing_statement() {
    let output = run_source(
        r#"
Sub Main()
    Dim x As Integer
    On Error GoTo Handler
    x = 1 / 0
    Console.WriteLine("after")
    GoTo Done
Handler:
    Console.WriteLine("handled")
    Resume Next
Done:
    Console.WriteLine("done")
End Sub
"#,
    );

    assert_eq!(output, vec!["handled", "after", "done"]);
}

#[test]
fn resume_label_jumps_to_requested_label() {
    let output = run_source(
        r#"
Sub Main()
    Dim x As Integer
    On Error GoTo Handler
    x = 1 / 0
    Console.WriteLine("after")
    GoTo Done
Handler:
    Console.WriteLine("handled")
    Resume ContinueHere
    Console.WriteLine("skip")
ContinueHere:
    Console.WriteLine("continued")
Done:
End Sub
"#,
    );

    assert_eq!(output, vec!["handled", "continued"]);
}

#[test]
fn on_error_and_resume_labels_are_semantically_validated() {
    let unknown_on_error = source_error(
        r#"
Sub Main()
    On Error GoTo Missing
End Sub
"#,
    );
    assert!(unknown_on_error.contains("Label 'Missing' is not declared"));

    let unknown_resume = source_error(
        r#"
Sub Main()
Handler:
    Resume Missing
End Sub
"#,
    );
    assert!(unknown_resume.contains("Label 'Missing' is not declared"));
}

#[test]
fn resume_without_active_handled_error_reports_runtime_diagnostic() {
    let error = source_error(
        r#"
Sub Main()
    Resume
End Sub
"#,
    );

    assert!(error.contains("Resume is only valid after a handled runtime error"));
}

#[test]
fn err_raise_is_handled_by_resume_next_and_goto_label() {
    let output = run_source(
        r#"
Sub Main()
    On Error Resume Next
    Err.Raise(7, "next", "resume next")
    Console.WriteLine(Err.Number)
    Console.WriteLine(Err.Description)
    Err.Clear()

    On Error GoTo Handler
    Err.Raise(8, "handler", "jumped")
    Console.WriteLine("skip")
    GoTo Done
Handler:
    Console.WriteLine(Err.Number)
    Console.WriteLine(Err.Source)
    Console.WriteLine(Err.Description)
Done:
End Sub
"#,
    );

    assert_eq!(output, vec!["7", "resume next", "8", "handler", "jumped"]);
}

#[test]
fn unhandled_err_raise_becomes_runtime_diagnostic() {
    let error = source_error(
        r#"
Sub Main()
    Err.Raise(77, "Unit.Test", "raised without handler")
End Sub
"#,
    );

    assert!(error.contains("raised without handler"));
}

#[test]
fn resume_after_on_error_goto_minus_one_fails() {
    let error = source_error(
        r#"
Sub Main()
    On Error GoTo Handler
    Err.Raise(1)
    GoTo Done
Handler:
    On Error GoTo -1
    Resume Next
Done:
End Sub
"#,
    );

    assert!(error.contains("Resume is only valid after a handled runtime error"));
}

#[test]
fn numeric_labels_goto_and_resume_work() {
    let output = run_source(
        r#"
Sub Main()
10 Console.WriteLine("start")
20 GoTo 40
30 Console.WriteLine("skip")
40 Console.WriteLine("done")

    On Error GoTo 70
50 Err.Raise(5)
60 Console.WriteLine("after")
    GoTo 80
70 Resume 60
80 Console.WriteLine("finished")
End Sub
"#,
    );

    assert_eq!(output, vec!["start", "done", "after", "finished"]);
}

#[test]
fn reports_duplicate_sub_function_name_conflict() {
    let error = source_error(
        r#"
Sub Same()
End Sub

Function Same() As Integer
    Return 1
End Function

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Name 'Same' conflicts with existing Sub"));
}

#[test]
fn creates_class_instance_and_calls_constructor() {
    let output = run_source(
        r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn class_method_mutation_persists_and_assignment_is_reference_like() {
    let output = run_source(
        r#"
Class User
    Public Name As String

    Public Sub Rename(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim a As User
    Dim b As User
    a = New User()
    b = a
    b.Rename("Changed")
    Console.WriteLine(a.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Changed"]);
}

#[test]
fn class_function_method_returns_value() {
    let output = run_source(
        r#"
Class User
    Private Age As Integer

    Public Sub Initialize(ByVal age As Integer)
        Me.Age = age
    End Sub

    Public Function IsAdult() As Boolean
        Return Me.Age >= 18
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User(20)
    Console.WriteLine(user.IsAdult())
End Sub
"#,
    );

    assert_eq!(output, vec!["True"]);
}

#[test]
fn private_field_access_outside_class_is_rejected() {
    let error = source_error(
        r#"
Class User
    Private Age As Integer
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Age)
End Sub
"#,
    );

    assert!(error.contains("Member 'Age' is Private in Class 'User'"));
}

#[test]
fn private_method_call_through_me_is_allowed() {
    let output = run_source(
        r#"
Class User
    Private Active As Boolean

    Private Sub SetActive(ByVal value As Boolean)
        Me.Active = value
    End Sub

    Public Sub Activate()
        Me.SetActive(True)
    End Sub

    Public Function IsActive() As Boolean
        Return Me.Active
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Activate()
    Console.WriteLine(user.IsActive())
End Sub
"#,
    );

    assert_eq!(output, vec!["True"]);
}

#[test]
fn private_method_call_outside_class_is_rejected() {
    let error = source_error(
        r#"
Class User
    Private Sub Hide()
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Hide()
End Sub
"#,
    );

    assert!(error.contains("Member 'Hide' is Private in Class 'User'"));
}

#[test]
fn me_outside_class_is_rejected() {
    let error = source_error(
        r#"
Sub Main()
    Console.WriteLine(Me)
End Sub
"#,
    );

    assert!(error.contains("Me is only valid inside class methods"));
}

#[test]
fn property_read_calls_get() {
    let output = run_source(
        r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName & "!"
    End Property

    Public Sub SetName(ByVal value As String)
        Me.mName = value
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.SetName("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo!"]);
}

#[test]
fn property_assignment_calls_let() {
    let output = run_source(
        r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value & " Runtime"
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Valo"
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo Runtime"]);
}

#[test]
fn property_validation_logic_mutates_backing_field() {
    let output = run_source(
        r#"
Class User
    Private mAge As Integer

    Public Property Get Age() As Integer
        Return Me.mAge
    End Property

    Public Property Let Age(ByVal value As Integer)
        If value < 0 Then
            Me.mAge = 0
        Else
            Me.mAge = value
        End If
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Age = -1
    Console.WriteLine(user.Age)
End Sub
"#,
    );

    assert_eq!(output, vec!["0"]);
}

#[test]
fn private_property_access_outside_class_is_rejected() {
    let error = source_error(
        r#"
Class User
    Private mName As String

    Private Property Get Name() As String
        Return Me.mName
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert!(error.contains("Member 'Name' is Private in Class 'User'"));
}

#[test]
fn private_property_access_inside_class_is_allowed() {
    let output = run_source(
        r#"
Class User
    Private mName As String

    Private Property Get Name() As String
        Return Me.mName
    End Property

    Private Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Sub Rename(ByVal value As String)
        Me.Name = value
    End Sub

    Public Function Label() As String
        Return Me.Name
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Rename("Valo")
    Console.WriteLine(user.Label())
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn missing_get_when_reading_property_produces_error() {
    let error = source_error(
        r#"
Class User
    Public Property Let Name(ByVal value As String)
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert!(error.contains("Property 'Name' has no Get accessor"));
}

#[test]
fn missing_let_or_set_when_assigning_property_produces_error() {
    let error = source_error(
        r#"
Class User
    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Runtime"
End Sub
"#,
    );

    assert!(error.contains("Property 'Name' has no Let or Set accessor"));
}

#[test]
fn duplicate_property_get_is_rejected() {
    let error = source_error(
        r#"
Class User
    Public Property Get Name() As String
        Return "a"
    End Property

    Public Property Get Name() As String
        Return "b"
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Property Get 'Name' is already declared"));
}

#[test]
fn duplicate_property_let_is_rejected() {
    let error = source_error(
        r#"
Class User
    Public Property Let Name(ByVal value As String)
    End Property

    Public Property Let Name(ByVal value As String)
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Property Let 'Name' is already declared"));
}

#[test]
fn property_conflicts_with_field_name() {
    let error = source_error(
        r#"
Class User
    Public Name As String

    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Property 'Name' conflicts with another member"));
}

#[test]
fn property_conflicts_with_method_name() {
    let error = source_error(
        r#"
Class User
    Public Sub Name()
    End Sub

    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Property 'Name' conflicts with another member"));
}

#[test]
fn property_get_missing_return_is_rejected() {
    let error = source_error(
        r#"
Class User
    Public Property Get Name() As String
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Property Get 'Name' must return a value"));
}

#[test]
fn property_let_with_wrong_parameter_count_is_rejected() {
    let error = source_error(
        r#"
Class User
    Public Property Let Name()
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("Property Let 'Name' must have exactly one parameter"));
}

#[test]
fn property_set_assigns_object_reference() {
    let output = run_source(
        r#"
Class Owner
    Public Name As String
End Class

Class Item
    Private mOwner As Owner

    Public Property Get Owner() As Owner
        Return Me.mOwner
    End Property

    Public Property Set Owner(ByVal value As Owner)
        Me.mOwner = value
    End Property
End Class

Sub Main()
    Dim owner As Owner
    Dim item As Item
    owner = New Owner()
    owner.Name = "Valo"
    item = New Item()
    item.Owner = owner
    Console.WriteLine(item.Owner.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn set_object_assignment_works() {
    let output = run_source(
        r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim user As User
    Set user = New User("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn set_me_field_assignment_works() {
    let output = run_source(
        r#"
Class Room
    Public North As String
End Class

Class Game
    Private mHall As Room

    Public Sub Initialize()
        Set Me.mHall = New Room()
        Me.mHall.North = "library"
    End Sub

    Public Function NorthExit() As String
        Return Me.mHall.North
    End Function
End Class

Sub Main()
    Dim game As Game
    game = New Game()
    Console.WriteLine(game.NorthExit())
End Sub
"#,
    );

    assert_eq!(output, vec!["library"]);
}

#[test]
fn set_object_field_and_nested_member_assignment_work() {
    let output = run_source(
        r#"
Class Child
    Public Name As String
End Class

Class Holder
    Public Child As Child
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    Set holder.Child = New Child()
    holder.Child.Name = "Valo"
    Console.WriteLine(holder.Child.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn set_object_property_dispatches_property_set() {
    let output = run_source(
        r#"
Class Child
    Public Name As String
End Class

Class Holder
    Private mChild As Child

    Public Property Get Child() As Child
        Return Me.mChild
    End Property

    Public Property Set Child(ByVal value As Child)
        Me.mChild = value
        Me.mChild.Name = "set"
    End Property
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    Set holder.Child = New Child()
    holder.Child.Name = holder.Child.Name & " ok"
    Console.WriteLine(holder.Child.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["set ok"]);
}

#[test]
fn nested_object_field_chain_assignment_works() {
    let output = run_source(
        r#"
Class Inner
    Public Value As String
End Class

Class Child
    Public Inner As Inner
End Class

Class Holder
    Public Child As Child
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    holder.Child = New Child()
    holder.Child.Inner = New Inner()
    holder.Child.Inner.Value = "deep"
    Console.WriteLine(holder.Child.Inner.Value)
End Sub
"#,
    );

    assert_eq!(output, vec!["deep"]);
}

#[test]
fn chained_assignment_reports_nothing_intermediate() {
    let error = source_error(
        r#"
Class Child
    Public Name As String
End Class

Class Holder
    Public Child As Child
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    holder.Child.Name = "Valo"
End Sub
"#,
    );

    assert!(error.contains("Object reference is Nothing"));
}

#[test]
fn not_is_precedence_matches_vba_style() {
    let output = run_source(
        r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    user = New User()

    If Not user Is Nothing Then
        user.Name = "Valo"
    End If

    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn not_is_other_object_precedence_works() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    Dim otherUser As User
    user = New User()
    otherUser = New User()

    If Not user Is otherUser Then
        Console.WriteLine("different")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["different"]);
}

#[test]
fn parenthesized_not_is_still_works() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    user = New User()

    If Not (user Is Nothing) Then
        Console.WriteLine("not nothing")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["not nothing"]);
}

#[test]
fn set_rejects_boolean_and_type_record_targets() {
    let boolean_error = source_error(
        r#"
Sub Main()
    Dim active As Boolean
    Set active = Nothing
End Sub
"#,
    );
    assert!(boolean_error.contains("Set target must be a class type"));

    let record_error = source_error(
        r#"
Type Point
    X As Integer
End Type

Sub Main()
    Dim point As Point
    Set point = Nothing
End Sub
"#,
    );
    assert!(record_error.contains("Set target must be a class type"));
}

#[test]
fn normal_object_assignment_still_works() {
    let output = run_source(
        r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    Dim aliasUser As User
    user = New User()
    user.Name = "Valo"
    aliasUser = user
    Console.WriteLine(aliasUser.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn class_variable_defaults_to_nothing() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    If user Is Nothing Then
        Console.WriteLine("empty")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["empty"]);
}

#[test]
fn field_access_on_nothing_errors_clearly() {
    let error = source_error(
        r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert!(error.contains("Object reference is Nothing"));
}

#[test]
fn method_call_on_nothing_errors_clearly() {
    let error = source_error(
        r#"
Class User
    Public Sub Rename(ByVal value As String)
    End Sub
End Class

Sub Main()
    Dim user As User
    user.Rename("Valo")
End Sub
"#,
    );

    assert!(error.contains("Object reference is Nothing"));
}

#[test]
fn set_user_to_nothing() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    Set user = New User()
    Set user = Nothing
    If user Is Nothing Then
        Console.WriteLine("empty")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["empty"]);
}

#[test]
fn is_nothing_false_after_new() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    Set user = New User()
    If Not (user Is Nothing) Then
        Console.WriteLine("present")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["present"]);
}

#[test]
fn object_identity_with_is() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    Dim aliasUser As User
    Set user = New User()
    Set aliasUser = user
    If user Is aliasUser Then
        Console.WriteLine("same")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["same"]);
}

#[test]
fn set_rejected_for_integer() {
    let error = source_error(
        r#"
Sub Main()
    Dim value As Integer
    Set value = 1
End Sub
"#,
    );

    assert!(error.contains("Set target must be a class type"));
}

#[test]
fn set_rejected_for_string() {
    let error = source_error(
        r#"
Sub Main()
    Dim value As String
    Set value = "Valo"
End Sub
"#,
    );

    assert!(error.contains("Set target must be a class type"));
}

#[test]
fn nothing_rejected_for_builtin_values() {
    for source in [
        r#"
Sub Main()
    Dim value As Integer
    value = Nothing
End Sub
"#,
        r#"
Sub Main()
    Dim value As String
    value = Nothing
End Sub
"#,
        r#"
Sub Main()
    Dim value As Boolean
    value = Nothing
End Sub
"#,
    ] {
        let error = source_error(source);
        assert!(error.contains("Nothing requires a class object type"));
    }
}

#[test]
fn nothing_rejected_for_type_records() {
    let error = source_error(
        r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As User
    user = Nothing
End Sub
"#,
    );

    assert!(error.contains("Nothing requires a class object type"));
}

#[test]
fn bare_sub_method_and_call_statement_syntax_work() {
    let output = run_source(
        r#"
Class User
    Public Name As String

    Public Sub SetName(ByVal value As String)
        Me.Name = value
    End Sub
End Class

Sub PrintMessage(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Ping()
    Console.WriteLine("ping")
End Sub

Sub Main()
    Dim user As User
    user = New User()
    PrintMessage "bare"
    Call PrintMessage("call parens")
    Call Ping
    user.SetName "Valo"
    Console.WriteLine(user.Name)
    Call user.SetName("Runtime")
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec!["bare", "call parens", "ping", "Valo", "Runtime"]
    );
}

#[test]
fn default_properties_are_used_for_output_concat_and_with_access() {
    let output = run_source(
        r#"
Class Person
    Private mName As String

    Public Sub Initialize(ByVal value As String)
        Me.mName = value
    End Sub

    Public Default Property Get Value() As String
        Return Me.mName
    End Property

    Public Property Let Value(ByVal value As String)
        Me.mName = value
    End Property
End Class

Sub Main()
    Dim p As Person
    p = New Person("Valo")
    Console.WriteLine(p)
    Console.WriteLine("name=" & p)
    With p
        .Value = "Runtime"
        Console.WriteLine(.Value)
    End With
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "name=Valo", "Runtime"]);
}

#[test]
fn duplicate_default_properties_are_rejected() {
    let error = source_error(
        r#"
Class Bad
    Public Default Property Get One() As String
        Return "one"
    End Property

    Public Default Property Get Two() As String
        Return "two"
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("multiple default members"));
}

#[test]
fn named_arguments_work_for_functions_subs_methods_and_constructors() {
    let output = run_source(
        r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub

    Public Sub SetName(ByVal title As String, ByVal name As String)
        Me.Name = title & " " & name
    End Sub
End Class

Sub Greet(ByVal title As String, ByVal name As String)
    Console.WriteLine(title & " " & name)
End Sub

Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Dim user As User
    user = New User(name := "Valo")
    Console.WriteLine(user.Name)
    Greet name := "Valo", title := "Runtime"
    Call Greet(name := "Valo", title := "Call")
    user.SetName name := "Valo", title := "Method"
    Console.WriteLine(user.Name)
    Console.WriteLine(Add(b := 20, a := 10))
End Sub
"#,
    );

    assert_eq!(
        output,
        vec!["Valo", "Runtime Valo", "Call Valo", "Method Valo", "30"]
    );
}

#[test]
fn typeof_is_checks_exact_class_and_nothing() {
    let output = run_source(
        r#"
Class User
End Class

Class Account
End Class

Sub Main()
    Dim user As User
    Dim account As Account
    user = New User()

    Console.WriteLine(TypeOf user Is User)
    Console.WriteLine(TypeOf account Is Account)
    Console.WriteLine(TypeOf user Is Account)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False", "False"]);
}

#[test]
fn typeof_rejects_scalar_and_unknown_class() {
    let scalar = source_error(
        r#"
Class User
End Class

Sub Main()
    Dim value As Integer
    Console.WriteLine(TypeOf value Is User)
End Sub
"#,
    );
    assert!(scalar.contains("TypeOf requires a class object"));

    let unknown = source_error(
        r#"
Sub Main()
    Dim value As Variant
    Console.WriteLine(TypeOf value Is MissingClass)
End Sub
"#,
    );
    assert!(unknown.contains("Class 'MissingClass' is not defined"));
}

#[test]
fn exit_function_in_object_function_returns_nothing() {
    let output = run_source(
        r#"
Class User
End Class

Function Value() As User
    Exit Function
    Return New User()
End Function

Sub Main()
    If Value() Is Nothing Then
        Console.WriteLine("nothing")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["nothing"]);
}

#[test]
fn with_blocks_support_members_methods_nesting_and_control_flow() {
    let output = run_source(
        r#"
Class Profile
    Public Name As String
End Class

Class User
    Private mName As String
    Public Profile As Profile

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Sub Activate()
        Me.mName = Me.mName & "!"
    End Sub

    Public Function Label() As String
        With Me
            Return .Name
        End With
        Return "bad"
    End Function

    Public Sub StopEarly()
        With Me
            Exit Sub
        End With
        Me.mName = "bad"
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Profile = New Profile()
    With user
        .Name = "Valo"
        Call .Activate()
        .Profile.Name = .Name
        With .Profile
            .Name = .Name & " Runtime"
        End With
        Console.WriteLine(.Profile.Name)
    End With
    Console.WriteLine(user.Label())
    user.StopEarly()
    Console.WriteLine(user.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo! Runtime", "Valo!", "Valo!"]);
}

#[test]
fn with_reports_dot_outside_nothing_and_evaluates_target_once() {
    let dot_error = source_error(
        r#"
Sub Main()
    Console.WriteLine(.Name)
End Sub
"#,
    );
    assert!(dot_error.contains("Dot member access requires an active With block"));

    let nothing_error = source_error(
        r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    With user
        .Name = "Valo"
    End With
End Sub
"#,
    );
    assert!(nothing_error.contains("Object reference is Nothing"));

    let output = run_source(
        r#"
Private calls As Integer

Class User
    Public Name As String
End Class

Function MakeUser() As User
    calls = calls + 1
    Return New User()
End Function

Sub Main()
    With MakeUser()
        .Name = "Valo"
        Console.WriteLine(.Name)
    End With
    Console.WriteLine(calls)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "1"]);
}

#[test]
fn let_and_call_statements_reuse_existing_assignment_and_sub_logic() {
    let output = run_source(
        r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Private Sub Mark()
        Me.mName = Me.mName & "!"
    End Sub

    Public Sub Touch()
        Call Me.Mark()
    End Sub
End Class

Sub PrintMessage(ByVal value As String)
    Console.WriteLine(value)
End Sub

Function Bad() As Integer
    Return 1
End Function

Sub Main()
    Dim values(0) As String
    Dim user As User
    user = New User()
    Let values(0) = "Valo"
    Let user.Name = values(0)
    With user
        Let .Name = .Name & " Runtime"
        Call .Touch()
    End With
    Call PrintMessage(user.Name)
    PrintMessage("plain")
End Sub
"#,
    );
    assert_eq!(output, vec!["Valo Runtime!", "plain"]);

    let call_function = source_error(
        r#"
Function Bad() As Integer
    Return 1
End Function

Sub Main()
    Call Bad()
End Sub
"#,
    );
    assert!(call_function.contains("Function 'Bad' cannot be called as a statement"));

    let unknown = source_error(
        r#"
Sub Main()
    Call Missing()
End Sub
"#,
    );
    assert!(unknown.contains("Sub 'Missing' is not defined"));
}

#[test]
fn class_event_raiseevent_without_handlers_does_nothing() {
    let output = run_source(
        r#"
Class Button
    Public Event Click(ByVal x As Integer, ByVal y As Integer)

    Public Sub Press()
        RaiseEvent Click(10, 20)
    End Sub
End Class

Sub Main()
    Dim button As Button
    button = New Button()
    button.Press()
    Console.WriteLine("done")
End Sub
"#,
    );

    assert_eq!(output, vec!["done"]);
}

#[test]
fn withevents_handler_is_invoked_and_receives_args() {
    let output = run_source(
        r#"
Class Button
    Public Event Click(ByVal x As Integer, ByVal y As Integer)

    Public Sub Press()
        RaiseEvent Click(10, 20)
    End Sub
End Class

Class Form
    Private WithEvents mButton As Button

    Public Sub Initialize()
        mButton = New Button()
    End Sub

    Private Sub mButton_Click(ByVal x As Integer, ByVal y As Integer)
        Console.WriteLine("clicked " & x & "," & y)
    End Sub

    Public Sub Run()
        mButton.Press()
    End Sub
End Class

Sub Main()
    Dim form As Form
    form = New Form()
    form.Run()
End Sub
"#,
    );

    assert_eq!(output, vec!["clicked 10,20"]);
}

#[test]
fn withevents_nothing_unbinds_and_reassignment_rebinds() {
    let output = run_source(
        r#"
Class Button
    Public Event Click(ByVal value As Integer)

    Public Sub Press(ByVal value As Integer)
        RaiseEvent Click(value)
    End Sub
End Class

Class Form
    Private WithEvents mButton As Button
    Private oldButton As Button

    Public Sub Run()
        mButton = New Button()
        oldButton = mButton
        oldButton.Press(1)
        mButton = Nothing
        oldButton.Press(2)
        mButton = New Button()
        oldButton.Press(3)
        mButton.Press(4)
    End Sub

    Private Sub mButton_Click(ByVal value As Integer)
        Console.WriteLine(value)
    End Sub
End Class

Sub Main()
    Dim form As Form
    form = New Form()
    form.Run()
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "4"]);
}

#[test]
fn rejects_wrong_event_handler_signature() {
    let error = source_error(
        r#"
Class Button
    Public Event Click(ByVal x As Integer)
End Class

Class Form
    Private WithEvents mButton As Button

    Private Sub mButton_Click(ByVal x As String)
    End Sub
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("signature does not match event 'Click'"));
}

#[test]
fn rejects_invalid_event_usage() {
    let unknown = source_error(
        r#"
Class Button
    Public Event Click()

    Public Sub Press()
        RaiseEvent Missing()
    End Sub
End Class

Sub Main()
End Sub
"#,
    );
    assert!(unknown.contains("has no event 'Missing'"));

    let outside = source_error(
        r#"
Sub Main()
    RaiseEvent Click()
End Sub
"#,
    );
    assert!(outside.contains("RaiseEvent is only valid inside the declaring class"));

    let direct = source_error(
        r#"
Class Button
    Public Event Click()
End Class

Sub Main()
    Dim button As Button
    button = New Button()
    button.Click()
End Sub
"#,
    );
    assert!(direct.contains("Event 'Click' cannot be called directly"));
}

#[test]
fn rejects_withevents_non_object_field() {
    let error = source_error(
        r#"
Class Form
    Private WithEvents counter As Integer
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("WithEvents field 'counter' must have a class type"));
}
