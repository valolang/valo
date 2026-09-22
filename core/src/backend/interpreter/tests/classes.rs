use crate::backend::interpreter::tests::helpers::*;
use crate::runtime::SourceMap;

#[test]
fn class_inheritance_dispatches_inherited_and_overridden_members() {
    let output = run_source(
        r#"
Class Animal
    Public Name As String
    Public Shared Count As Long

    Public Overridable Sub Speak()
        Console.WriteLine("...")
    End Sub
End Class

Class Dog Inherits Animal
    Public Overrides Sub Speak()
        Console.WriteLine("Woof " & Name)
    End Sub
End Class

Sub Main()
    Dim a As Animal
    a = New Dog()
    a.Name = "Rex"
    Dog.Count = 3
    a.Speak()
    Console.WriteLine(Dog.Count)
    Console.WriteLine(TypeOf a Is Animal)
    Console.WriteLine(TypeOf a Is Dog)
End Sub
"#,
    );

    assert_eq!(output, vec!["Woof Rex", "3", "True", "True"]);
}

#[test]
fn mustinherit_and_mustoverride_require_concrete_override() {
    let error = source_error(
        r#"
MustInherit Class Animal
    Public MustOverride Sub Speak()
    End Sub
End Class

Class Dog Inherits Animal
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("must override inherited MustOverride member"));
}

#[test]
fn notinheritable_class_cannot_be_inherited() {
    let error = source_error(
        r#"
NotInheritable Class Utility
End Class

Class Child Inherits Utility
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains("cannot inherit NotInheritable class"));
}

#[test]
fn mybase_sub_call_bypasses_override_dispatch() {
    let output = run_source(
        r#"
Class Animal
    Public Overridable Sub Speak()
        Console.WriteLine("base")
    End Sub
End Class

Class Dog Inherits Animal
    Public Overrides Sub Speak()
        MyBase.Speak()
        Console.WriteLine("dog")
    End Sub
End Class

Sub Main()
    Dim d As New Dog()
    d.Speak()
End Sub
"#,
    );

    assert_eq!(output, vec!["base", "dog"]);
}

#[test]
fn generic_base_class_members_are_inherited() {
    let output = run_source(
        r#"
Class Box(Of T)
    Public Value As T
End Class

Class StringBox Inherits Box(Of String)
End Class

Sub Main()
    Dim box As New StringBox()
    box.Value = "ok"
    Console.WriteLine(box.Value)
End Sub
"#,
    );

    assert_eq!(output, vec!["ok"]);
}

#[test]
fn default_member_as_new_and_constructor_work() {
    let output = run_source(
        r#"

Class Box
    Private stored As Integer

    Public Sub New()
        stored = 11
    End Sub

    Public ReadOnly Default Property Value() As Integer
        Get
        Value = stored
        End Get
    End Property
End Class

Function MakeBox() As Object
    MakeBox = New Box()
End Function

Sub Main()
    Dim a As New Box
    Console.WriteLine(a)
    Console.WriteLine(IsObject(a))
    Dim b As Object
    b = MakeBox()
    Console.WriteLine(TypeName(b))
End Sub
"#,
    );

    assert_eq!(output, vec!["11", "True", "Box"]);
}

#[test]
fn redim_bare_variant_class_field_resizes_and_persists() {
    let output = run_source(
        r#"
Class Inventory
    Private names As Variant

    Public Sub New()
        ReDim names(0 To 2)
        names(0) = "bolt"
        names(1) = "nut"
        names(2) = "washer"
    End Sub

    Public Function Item(ByVal index As Integer) As Variant
        Item = names(index)
    End Function
End Class

Sub Main()
    Dim inventory As New Inventory
    Console.WriteLine(inventory.Item(0))
    Console.WriteLine(inventory.Item(2))
End Sub
"#,
    );

    assert_eq!(output, vec!["bolt", "washer"]);
}

#[test]
fn redim_preserve_bare_class_field_keeps_contents() {
    let output = run_source(
        r#"
Class Inventory
    Private names As Variant

    Public Sub New()
        ReDim names(0 To 1)
        names(0) = "first"
        names(1) = "second"
        ReDim Preserve names(0 To 3)
        names(2) = "third"
    End Sub

    Public Function Item(ByVal index As Integer) As Variant
        Item = names(index)
    End Function

    Public Function Count() As Integer
        Count = UBound(names) + 1
    End Function
End Class

Sub Main()
    Dim inventory As New Inventory
    Console.WriteLine(inventory.Item(0))
    Console.WriteLine(inventory.Item(1))
    Console.WriteLine(inventory.Item(2))
    Console.WriteLine(inventory.Count())
End Sub
"#,
    );

    assert_eq!(output, vec!["first", "second", "third", "4"]);
}

#[test]
fn redim_me_field_supported_for_variant_class_field() {
    let output = run_source(
        r#"
Class Inventory
    Private names As Variant

    Public Sub New()
        ReDim Me.names(0 To 0)
        names(0) = "direct"
    End Sub

    Public Function First() As Variant
        First = names(0)
    End Function
End Class

Sub Main()
    Dim inventory As New Inventory
    Console.WriteLine(inventory.First())
End Sub
"#,
    );

    assert_eq!(output, vec!["direct"]);
}

#[test]
fn dynamic_variant_class_field_matches_exported_cls_items_pattern() {
    let output = run_source(
        r#"
Class Bag
    Private pItems() As Variant
    Private pCapacity As Long

    Private Sub Class_Initialize()
        pCapacity = 8
        ReDim pItems(0 To pCapacity - 1)
        pItems(0) = "seed"
        ReDim Preserve pItems(0 To pCapacity)
        pItems(8) = "extra"
    End Sub

    Public Function First() As Variant
        First = pItems(0)
    End Function

    Public Function Last() As Variant
        Last = pItems(8)
    End Function
End Class

Sub Main()
    Dim bag As New Bag
    Console.WriteLine(bag.First())
    Console.WriteLine(bag.Last())
End Sub
"#,
    );

    assert_eq!(output, vec!["seed", "extra"]);
}

#[test]
fn set_assignment_to_variant_field_array_element_works() {
    let output = run_source(
        r#"
Class Item
    Public Name As String
End Class

Class Bag
    Private pItems() As Variant

    Private Sub Class_Initialize()
        ReDim pItems(0 To 0)
        pItems(0) = New Item()
        pItems(0).Name = "stored"
    End Sub

    Public Function FirstName() As String
        FirstName = pItems(0).Name
    End Function
End Class

Sub Main()
    Dim bag As New Bag
    Console.WriteLine(bag.FirstName())
End Sub
"#,
    );

    assert_eq!(output, vec!["stored"]);
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
    let mut map = SourceMap::new();
    map.add("stack.valo".to_string(), source.to_string());
    let rendered = diagnostic.render_colored(&map, false);

    assert!(rendered.contains("error[V1200]"));
    assert!(rendered.contains("note: while executing Sub 'Boom'"));
    assert!(rendered.contains("Stack trace:"));
    assert!(rendered.contains("at Sub 'Boom'"));
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

    Public Sub New(ByVal name As String)
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

    Public Sub New(ByVal age As Integer)
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

    Public ReadOnly Property Name() As String
        Get
        Return Me.mName & "!"
        End Get
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

    Public Property Name() As String
        Get
        Return Me.mName
        End Get
        Set(ByVal value As String)
        Me.mName = value & " Runtime"
        End Set
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

    Public Property Age() As Integer
        Get
        Return Me.mAge
        End Get
        Set(ByVal value As Integer)
        If value < 0 Then
            Me.mAge = 0
        Else
            Me.mAge = value
        End If
        End Set
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
fn vbnet_auto_properties_support_defaults_get_and_set() {
    let output = run_source(
        r#"
Class Product
    Public Property Id As Integer
    Public Property Name As String = "Unnamed Product"
End Class

Sub Main()
    Dim product As New Product()
    Console.WriteLine(product.Id)
    Console.WriteLine(product.Name)
    product.Id = 7
    product.Name = "Valo"
    Console.WriteLine(product.Id)
    Console.WriteLine(product.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "Unnamed Product", "7", "Valo"]);
}

#[test]
fn vbnet_full_property_blocks_support_get_and_set_validation() {
    let output = run_source(
        r#"
Class Account
    Private _balance As Decimal

    Public Property Balance As Decimal
        Get
            Return _balance
        End Get
        Set(value As Decimal)
            If value >= 0 Then
                _balance = value
            Else
                _balance = 0
            End If
        End Set
    End Property
End Class

Sub Main()
    Dim account As New Account()
    account.Balance = 10
    Console.WriteLine(account.Balance)
    account.Balance = -1
    Console.WriteLine(account.Balance)
End Sub
"#,
    );

    assert_eq!(output, vec!["10", "0"]);
}

#[test]
fn vbnet_readonly_and_writeonly_properties_enforce_accessors() {
    let output = run_source(
        r#"
Class Config
    Private _senha As String

    Public ReadOnly Property Versao As String
        Get
            Return "1.0.0"
        End Get
    End Property

    Public WriteOnly Property Senha As String
        Set(value As String)
            _senha = value
        End Set
    End Property

    Public Function Masked() As String
        Return "***"
    End Function
End Class

Sub Main()
    Dim config As New Config()
    Console.WriteLine(config.Versao)
    config.Senha = "secret"
    Console.WriteLine(config.Masked())
End Sub
"#,
    );

    assert_eq!(output, vec!["1.0.0", "***"]);

    let read_only_error = source_error(
        r#"
Class Config
    Public ReadOnly Property Versao As String
        Get
            Return "1.0.0"
        End Get
    End Property
End Class

Sub Main()
    Dim config As New Config()
    config.Versao = "2.0.0"
End Sub
"#,
    );
    assert!(read_only_error.contains("Property 'Versao' has no Set accessor"));

    let write_only_error = source_error(
        r#"
Class Config
    Public WriteOnly Property Senha As String
        Set(value As String)
        End Set
    End Property
End Class

Sub Main()
    Dim config As New Config()
    Console.WriteLine(config.Senha)
End Sub
"#,
    );
    assert!(write_only_error.contains("Property 'Senha' has no Get accessor"));
}

#[test]
fn private_property_access_outside_class_is_rejected() {
    let error = source_error(
        r#"
Class User
    Private mName As String

    Private ReadOnly Property Name() As String
        Get
        Return Me.mName
        End Get
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

    Private Property Name() As String
        Get
        Return Me.mName
        End Get
        Set(ByVal value As String)
        Me.mName = value
        End Set
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
    Public WriteOnly Property Name() As String
        Set(ByVal value As String)
        End Set
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
    Public ReadOnly Property Name() As String
        Get
        Return "Valo"
        End Get
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Runtime"
End Sub
"#,
    );

    assert!(error.contains("Property 'Name' has no Set accessor"));
}

#[test]
fn duplicate_property_get_is_rejected() {
    let error = source_error(
        r#"
Class User
    Public ReadOnly Property Name() As String
        Get
        Return "a"
        End Get
    End Property

    Public ReadOnly Property Name() As String
        Get
        Return "b"
        End Get
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains(
        "Property Get 'Name' in Class 'User' is already declared with these parameter types"
    ));
}

#[test]
fn duplicate_property_let_is_rejected() {
    let error = source_error(
        r#"
Class User
    Public WriteOnly Property Name() As String
        Set(ByVal value As String)
        End Set
    End Property

    Public WriteOnly Property Name() As String
        Set(ByVal value As String)
        End Set
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(error.contains(
        "Property Set 'Name' in Class 'User' is already declared with these parameter types"
    ));
}

#[test]
fn property_conflicts_with_field_name() {
    let error = source_error(
        r#"
Class User
    Public Name As String

    Public ReadOnly Property Name() As String
        Get
        Return "Valo"
        End Get
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

    Public ReadOnly Property Name() As String
        Get
        Return "Valo"
        End Get
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
    Public ReadOnly Property Name() As String
        Get
        End Get
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

    assert!(error.contains("Legacy Property Get/Let/Set declarations have been removed"));
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

    Public Property Owner() As Owner
        Get
        Return Me.mOwner
        End Get
        Set(ByVal value As Owner)
        Me.mOwner = value
        End Set
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

    Public Sub New(ByVal name As String)
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
fn set_me_field_assignment_works() {
    let output = run_source(
        r#"
Class Room
    Public North As String
End Class

Class Game
    Private mHall As Room

    Public Sub New()
        Me.mHall = New Room()
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
    holder.Child = New Child()
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

    Public Property Child() As Child
        Get
        Return Me.mChild
        End Get
        Set(ByVal value As Child)
        Me.mChild = value
        Me.mChild.Name = "set"
        End Set
    End Property


End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    holder.Child = New Child()
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
fn unknown_method_lists_available_members_and_suggestion() {
    let diagnostic = source_diagnostic(
        r#"
Class Bag
Public Sub Add()
End Sub
Public Function Count() As Integer
    Count = 0
End Function
End Class

Sub Main()
    Dim bag As New Bag
    bag.Cuont()
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::MEMBER_ACCESS
    );
    assert!(
        diagnostic
            .helps
            .iter()
            .any(|help| help.contains("did you mean 'Count'?"))
    );
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("available members") && note.contains("Add"))
    );
}

#[test]
fn set_user_to_nothing() {
    let output = run_source(
        r#"
Class User
End Class

Sub Main()
    Dim user As User
    user = New User()
    user = Nothing
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
    user = New User()
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
    user = New User()
    aliasUser = user
    If user Is aliasUser Then
        Console.WriteLine("same")
    End If
End Sub
"#,
    );

    assert_eq!(output, vec!["same"]);
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

    Public Sub New(ByVal value As String)
        Me.mName = value
    End Sub

    Public Default Property Value() As String
        Get
        Return Me.mName
        End Get
        Set(ByVal value As String)
        Me.mName = value
        End Set
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
    Public ReadOnly Default Property One() As String
        Get
        Return "one"
        End Get
    End Property

    Public ReadOnly Default Property Two() As String
        Get
        Return "two"
        End Get
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

    Public Sub New(ByVal name As String)
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

    Public Property Name() As String
        Get
        Return Me.mName
        End Get
        Set(ByVal value As String)
        Me.mName = value
        End Set
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

    Public Property Name() As String
        Get
        Return Me.mName
        End Get
        Set(ByVal value As String)
        Me.mName = value
        End Set
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

    let call_function = run_source(
        r#"
Function Bad() As Integer
    Console.WriteLine("called")
    Return 1
End Function

Sub Main()
    Call Bad()
End Sub
"#,
    );
    assert_eq!(call_function, vec!["called"]);

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

    Public Sub New()
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
fn test_throw_argument_exception() {
    let output = run_source(
        r#"
Class Account
    Private _balance As Decimal

    Public Property Balance As Decimal
        Get
            Return _balance
        End Get
        Set(value As Decimal)
            If value >= 0 Then
                _balance = value
            Else
                Throw "Balance cannot be negative."
            End If
        End Set
    End Property
End Class

Sub Main()
    Dim account As New Account()
    Try
        account.Balance = 10
        Console.WriteLine(account.Balance)
        account.Balance = -1
    Catch ex As Error
        Console.WriteLine(ex.Message)
    End Try
End Sub
"#,
    );

    assert_eq!(output, vec!["10", "Balance cannot be negative."]);
}

#[test]
fn inherits_can_appear_on_its_own_line_inside_the_class_body() {
    let output = run_source(
        r#"
Class Animal
    Public Name As String

    Public Overridable Sub Speak()
        Console.WriteLine("...")
    End Sub
End Class

Class Dog
    Inherits Animal

    Public Overrides Sub Speak()
        Console.WriteLine(Name & " says woof")
    End Sub
End Class

Sub Main()
    Dim d As New Dog()
    d.Name = "Rex"
    d.Speak()
End Sub
"#,
    );

    assert_eq!(output, vec!["Rex says woof"]);
}

#[test]
fn object_initializer_sets_fields_after_construction() {
    let output = run_source(
        r#"
Class Point
    Public X As Long
    Public Y As Long
End Class

Sub Main()
    Dim p As Point = New Point With { .X = 3, .Y = 4 }
    Console.WriteLine(p.X & "," & p.Y)

    Dim q As New Point With { .X = 7, .Y = 8 }
    Console.WriteLine(q.X & "," & q.Y)
End Sub
"#,
    );

    assert_eq!(output, vec!["3,4", "7,8"]);
}

#[test]
fn object_initializer_spans_lines_and_allows_a_trailing_comma() {
    let output = run_source(
        r#"
Class Point
    Public X As Long
    Public Y As Long
End Class

Sub Main()
    Dim p As Point = New Point With {
        .X = 10,
        .Y = 20,
    }
    Console.WriteLine(p.X & "," & p.Y)
End Sub
"#,
    );

    assert_eq!(output, vec!["10,20"]);
}

#[test]
fn object_initializer_runs_property_setters() {
    let output = run_source(
        r#"
Class Person
    Private storedName As String

    Public Property Name As String
        Get
            Return storedName
        End Get
        Set(ByVal value As String)
            storedName = UCase(value)
        End Set
    End Property
End Class

Sub Main()
    Dim who As Person = New Person With { .Name = "ada" }
    Console.WriteLine(who.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["ADA"]);
}

#[test]
fn object_initializer_rejects_an_unknown_member() {
    let diagnostic = source_diagnostic(
        r#"
Class Point
    Public X As Long
End Class

Sub Main()
    Dim p As Point = New Point With { .Missing = 1 }
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::MEMBER_ACCESS
    );
}

#[test]
fn object_initializer_rejects_a_mistyped_value_and_a_repeated_member() {
    let mistyped = source_diagnostic(
        r#"
Class Point
    Public X As Long
End Class

Sub Main()
    Dim p As Point = New Point With { .X = "not a number" }
End Sub
"#,
    );
    assert_eq!(mistyped.code, crate::runtime::DiagnosticCode::TYPE_MISMATCH);

    let repeated = source_diagnostic(
        r#"
Class Point
    Public X As Long
End Class

Sub Main()
    Dim p As Point = New Point With { .X = 1, .X = 2 }
End Sub
"#,
    );
    assert_eq!(
        repeated.code,
        crate::runtime::DiagnosticCode::DUPLICATE_DECLARATION
    );
}

#[test]
fn null_conditional_access_yields_nothing_for_a_missing_receiver() {
    let output = run_source(
        r#"
Class Address
    Public City As String
End Class

Class Customer
    Public Home As Address
End Class

Sub Main()
    Dim known As New Customer()
    known.Home = New Address With { .City = "London" }

    Dim unknown As New Customer()

    Console.WriteLine(known.Home?.City)
    Console.WriteLine(unknown.Home?.City Is Nothing)
End Sub
"#,
    );

    assert_eq!(output, vec!["London", "True"]);
}

#[test]
fn null_conditional_guards_method_calls_and_the_whole_chain() {
    let output = run_source(
        r#"
Class Address
    Public City As String

    Public Function Label() As String
        Return "City: " & City
    End Function
End Class

Class Customer
    Public Home As Address
End Class

Sub Main()
    Dim known As New Customer()
    known.Home = New Address With { .City = "London" }

    Dim unknown As New Customer()
    Console.WriteLine(known.Home?.Label())
    Console.WriteLine(unknown.Home?.Label() Is Nothing)

    ' The guard covers everything after it, so `.City` is never reached.
    Dim missing As Customer = Nothing
    Console.WriteLine(missing?.Home.City Is Nothing)
End Sub
"#,
    );

    assert_eq!(output, vec!["City: London", "True", "True"]);
}

#[test]
fn a_null_conditional_access_cannot_be_assigned_to() {
    let diagnostic = source_diagnostic(
        r#"
Class Address
    Public City As String
End Class

Class Customer
    Public Home As Address
End Class

Sub Main()
    Dim c As New Customer()
    c.Home?.City = "London"
End Sub
"#,
    );

    assert_eq!(
        diagnostic.code,
        crate::runtime::DiagnosticCode::INVALID_ASSIGNMENT
    );
}

#[test]
fn a_property_read_from_another_instance_uses_that_instance() {
    let output = run_source(
        r#"
Class Box
    Public X As Double

    Public Property Edge As Double
        Get
            Return X + 1000
        End Get
    End Property

    ' Reading a property off another object of the same class must run the
    ' getter against that object, not against the receiver of this method.
    Public Function EdgeOf(ByVal other As Box) As Double
        Return other.Edge
    End Function
End Class

Sub Main()
    Dim first As New Box()
    first.X = 1
    Dim second As New Box()
    second.X = 2

    Console.WriteLine(first.EdgeOf(second))
    Console.WriteLine(first.Edge)
End Sub
"#,
    );

    assert_eq!(output, vec!["1002", "1001"]);
}

#[test]
fn comparing_two_instances_through_their_properties_is_correct() {
    let output = run_source(
        r#"
Class Span
    Public Start As Double
    Public Width As Double

    Public Property Finish As Double
        Get
            Return Start + Width
        End Get
    End Property

    Public Function IsLeftOf(ByVal other As Span) As Boolean
        Return Finish <= other.Start
    End Function
End Class

Sub Main()
    Dim near As New Span()
    near.Start = 0
    near.Width = 10

    Dim far As New Span()
    far.Start = 50
    far.Width = 10

    Console.WriteLine(near.IsLeftOf(far))
    Console.WriteLine(far.IsLeftOf(near))
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False"]);
}

#[test]
fn a_field_can_be_assigned_through_without_naming_me() {
    let output = run_source(
        r#"
Class Point
    Public X As Double
End Class

Class Holder
    Public Spot As Point

    Public Sub Initialize()
        Spot = New Point()
    End Sub

    Public Sub MoveViaField()
        Spot.X = 10
    End Sub

    Public Sub MoveViaMe()
        Me.Spot.X = 20
    End Sub

    Public Function Where() As Double
        Return Spot.X
    End Function
End Class

Sub Main()
    Dim holder As New Holder()

    holder.MoveViaMe()
    Console.WriteLine(holder.Where())

    holder.MoveViaField()
    Console.WriteLine(holder.Where())
End Sub
"#,
    );

    assert_eq!(output, vec!["20", "10"]);
}

#[test]
fn a_property_can_be_assigned_through_without_naming_me() {
    let output = run_source(
        r#"
Class Leaf
    Public N As Double
End Class

Class Holder
    Public Slot As Leaf

    Public Sub Initialize()
        Slot = New Leaf()
    End Sub

    Public Property Reached As Leaf
        Get
            Return Slot
        End Get
    End Property

    Public Sub Move(ByVal n As Double)
        Reached.N = n
    End Sub
End Class

Sub Main()
    Dim holder As New Holder()
    holder.Move(9)
    Console.WriteLine(holder.Slot.N)
End Sub
"#,
    );

    assert_eq!(output, vec!["9"]);
}

#[test]
fn set_resolves_through_a_nested_member() {
    let output = run_source(
        r#"
Class Leaf
    Public N As Double
End Class

Class Branch
    Public Tip As Leaf
End Class

Class Trunk
    Public Limb As Branch

    Public Sub Initialize()
        Limb = New Branch()
        Limb.Tip = New Leaf()
    End Sub
End Class

Sub Main()
    Dim tree As New Trunk()
    tree.Limb.Tip.N = 1
    Console.WriteLine(tree.Limb.Tip.N)

    Dim graft As New Leaf()
    graft.N = 2
    tree.Limb.Tip = graft
    Console.WriteLine(tree.Limb.Tip.N)
End Sub
"#,
    );

    assert_eq!(output, vec!["1", "2"]);
}

#[test]
fn class_methods_overload_by_argument_count_and_type() {
    let output = run_source(
        r#"
Class Painter
    Public Overloads Function Draw(ByVal x As Double) As String
        Return "point " & x
    End Function

    Public Overloads Function Draw(ByVal x As Double, ByVal y As Double) As String
        Return "line " & x & "," & y
    End Function

    Public Overloads Function Draw(ByVal label As String) As String
        Return "label " & label
    End Function

    Public Sub Trace(ByVal n As Long)
        Console.WriteLine("n " & n)
    End Sub

    Public Sub Trace(ByVal text As String)
        Console.WriteLine("s " & text)
    End Sub
End Class

Sub Main()
    Dim p As New Painter()
    Console.WriteLine(p.Draw(1))
    Console.WriteLine(p.Draw(1, 2))
    Console.WriteLine(p.Draw("here"))
    p.Trace(4)
    p.Trace("four")
End Sub
"#,
    );

    assert_eq!(
        output,
        vec!["point 1", "line 1,2", "label here", "n 4", "s four"]
    );
}

#[test]
fn a_constructor_can_be_overloaded() {
    let output = run_source(
        r#"
Class Painter
    Public Name_ As String

    Public Sub Initialize()
        Name_ = "unnamed"
    End Sub

    Public Sub Initialize(ByVal name As String)
        Name_ = name
    End Sub

    Public Shared Function Make() As Painter
        Return New Painter()
    End Function

    Public Shared Function Make(ByVal name As String) As Painter
        Return New Painter(name)
    End Function
End Class

Sub Main()
    Console.WriteLine(Painter.Make().Name_)
    Console.WriteLine(Painter.Make("studio").Name_)
End Sub
"#,
    );

    assert_eq!(output, vec!["unnamed", "studio"]);
}

#[test]
fn structure_methods_overload_too() {
    let output = run_source(
        r#"
Structure Box_
    Public W As Double

    Public Function Size() As Double
        Return W
    End Function

    Public Function Size(ByVal scale As Double) As Double
        Return W * scale
    End Function
End Structure

Sub Main()
    Dim b As Box_
    b.W = 10
    Console.WriteLine(b.Size())
    Console.WriteLine(b.Size(3))
End Sub
"#,
    );

    assert_eq!(output, vec!["10", "30"]);
}

#[test]
fn two_methods_with_the_same_parameters_are_rejected() {
    let diagnostic = source_diagnostic(
        r#"
Class Painter
    Public Sub Draw(ByVal x As Double)
    End Sub

    Public Sub Draw(ByVal y As Double)
    End Sub
End Class

Sub Main()
End Sub
"#,
    );

    assert!(diagnostic.message.contains(
        "Method 'Draw' in Class 'Painter' is already declared with these parameter types"
    ));
}

#[test]
fn an_indexed_property_can_be_written_as_well_as_read() {
    let output = run_source(
        r#"
Class Store
    Private Slots(9) As String

    Public Property Item(ByVal index As Long) As String
        Get
        Return Slots(index)
        End Get
        Set(ByVal value As String)
        Slots(index) = value
        End Set
    End Property


End Class

Sub Main()
    Dim s As New Store()
    s.Item(2) = "two"
    Console.WriteLine(s.Item(2))
End Sub
"#,
    );

    assert_eq!(output, vec!["two"]);
}

#[test]
fn property_accessors_overload_on_what_indexes_them() {
    let output = run_source(
        r#"
Class Store
    Private Slots(9) As String
    Private Named As String

    Public Property Item(ByVal index As Long) As String
        Get
        Return Slots(index)
        End Get
        Set(ByVal value As String)
        Slots(index) = value
        End Set
    End Property

    Public Property Item(ByVal key As String) As String
        Get
        Return Named
        End Get
        Set(ByVal value As String)
        Named = key & "=" & value
        End Set
    End Property




End Class

Sub Main()
    Dim s As New Store()
    s.Item(2) = "two"
    s.Item("k") = "v"
    Console.WriteLine(s.Item(2))
    Console.WriteLine(s.Item("k"))
End Sub
"#,
    );

    assert_eq!(output, vec!["two", "k=v"]);
}

#[test]
fn two_accessors_with_the_same_parameters_are_rejected() {
    let diagnostic = source_diagnostic(
        r#"
Class Store
    Public ReadOnly Property Item(ByVal index As Long) As String
        Get
        Return "a"
        End Get
    End Property

    Public ReadOnly Property Item(ByVal other As Long) As String
        Get
        Return "b"
        End Get
    End Property
End Class

Sub Main()
End Sub
"#,
    );

    assert!(diagnostic.message.contains(
        "Property Get 'Item' in Class 'Store' is already declared with these parameter types"
    ));
}

#[test]
fn a_getter_runs_once_when_the_member_is_called() {
    let output = run_source(
        r#"
Class Counter
    Public Reads As Long

    Public ReadOnly Property Touched() As Long
        Get
        Reads = Reads + 1
        Return Reads
        End Get
    End Property

    Public Function Look() As Long
        Return Reads
    End Function
End Class

Sub Main()
    Dim c As New Counter()
    Console.WriteLine(c.Touched)
    Console.WriteLine(c.Look())
    Console.WriteLine(c.Reads)
End Sub
"#,
    );

    // One read of the property, and calling a method does not touch it.
    assert_eq!(output, vec!["1", "1", "1"]);
}

#[test]
fn a_variable_shadows_a_class_of_the_same_name_as_a_receiver() {
    let output = run_source(
        r#"
Class Config
    Public Value_ As Long
    Public Shared Function Default_() As Long
        Return 99
    End Function
End Class

Sub Main()
    ' A local named after the class is the receiver here, not the class.
    Dim Config As New Config()
    Config.Value_ = 7
    Console.WriteLine(Config.Value_)
End Sub
"#,
    );

    assert_eq!(output, vec!["7"]);
}

#[test]
fn one_call_site_reaching_several_classes_runs_each_of_them() {
    let output = run_source(
        r#"
Interface IShape
    Function Name_() As String
End Interface

Class Circle
    Implements IShape
    Public Function Name_() As String Implements IShape.Name_
        Return "circle"
    End Function
End Class

Class Square
    Implements IShape
    Public Function Name_() As String Implements IShape.Name_
        Return "square"
    End Function
End Class

Sub Main()
    Dim shapes As New Collection()
    shapes.Add(New Circle())
    shapes.Add(New Square())
    shapes.Add(New Circle())

    ' One call site, three receivers, two classes between them.
    Dim item As Variant
    For Each item In shapes
        Dim shape As IShape
        shape = CType(item, IShape)
        Console.WriteLine(shape.Name_())
    Next item
End Sub
"#,
    );

    assert_eq!(output, vec!["circle", "square", "circle"]);
}
