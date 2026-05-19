use crate::interpreter::tests::helpers::*;

#[test]
fn native_default_property_works() {
    let output = run_source(
        r#"
Class box
    Private value As Integer
    Public Sub Initialize()
        Me.value = 10
    End Sub
    Public Default Property Get item() As Integer
        Return Me.value
    End Property
End Class
Sub Main()
    Dim b As box
    Set b = New box()
    Dim v As Variant
    v = b
    Console.WriteLine(v)
End Sub
"#.trim(),
    );

    assert_eq!(output, vec!["10"]);
}

#[test]
fn indexer_style_default_property_works() {
    let output = run_source(
        r#"
Class MyList
    Private m_items As Variant
    Public Sub Initialize()
    End Sub
    Public Default Property Get item(ByVal index As Integer) As String
        Return "val" & index
    End Property
End Class
Sub Main()
    Dim l As New MyList
    Dim v0 As String
    v0 = l(0)
    Console.WriteLine(v0)
    Dim v1 As String
    v1 = l(1)
    Console.WriteLine(v1)
End Sub
"#.trim(),
    );

    assert_eq!(output, vec!["val0", "val1"]);
}

#[test]
fn duplicate_default_property_rejected() {
    let error = source_error(
        r#"
Class Bad
    Public Default Property Get One() As Integer
        Return 1
    End Property
    Public Default Property Get Two() As Integer
        Return 2
    End Property
End Class
Sub Main()
End Sub
"#,
    );
    assert!(error.contains("multiple default members"));
}

#[test]
fn invalid_default_property_kind_rejected() {
    let error = source_error(
        r#"
Class Bad
    Public Default Property Let One(value As Integer)
    End Property
End Class
Sub Main()
End Sub
"#,
    );
    assert!(error.contains("Only Property Get can be marked as Default"));
}

#[test]
fn return_in_property_get_works() {
    let output = run_source(
        r#"
Class User
    Private mName As String
    Public Sub Initialize(ByVal name As String)
        Me.mName = name
    End Sub
    Public Property Get name() As String
        Return Me.mName
    End Property
End Class
Sub Main()
    Dim u As User
    Set u = New User("Valo")
    Dim n As String
    n = u.name
    Console.WriteLine(n)
End Sub
"#.trim(),
    );
    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn visibility_keywords_before_class_enum_type_work() {
    let output = run_source(
        r#"
Public Class User
    Public Name As String
End Class
Private Enum Color
    Red = 1
End Enum
Public Type Point
    X As Integer
End Type
Sub Main()
    Dim u As New User
    u.Name = "Valo"
    Console.WriteLine(u.Name)
    Console.WriteLine(Color.Red)
    Dim p As Point
    p.X = 10
    Console.WriteLine(p.X)
End Sub
"#,
    );
    assert_eq!(output, vec!["Valo", "1", "10"]);
}

#[test]
fn default_property_dispatch_on_member_call() {
    let output = run_source(
        r#"
Class Inner
    Public Default Property Get value() As String
        Return "Deep"
    End Property
End Class
Class Outer
    Private mInner As Inner
    Public Sub Initialize()
        Set Me.mInner = New Inner()
    End Sub
    Public Property Get info() As Inner
        Return Me.mInner
    End Property
End Class
Sub Main()
    Dim o As Outer
    Set o = New Outer()
    Dim v As Variant
    v = o.info
    Console.WriteLine(v)
End Sub
"#.trim(),
    );
    assert_eq!(output, vec!["Deep"]);
}

#[test]
fn default_property_with_arguments_dispatch_on_member_call() {
    let output = run_source(
        r#"
Class Inner
    Public Default Property Get item(ByVal idx As Integer) As String
        Return "Item " & idx
    End Property
End Class
Class Outer
    Private mInner As Inner
    Public Sub Initialize()
        Set Me.mInner = New Inner()
    End Sub
    Public Property Get collection() As Inner
        Return Me.mInner
    End Property
End Class
Sub Main()
    Dim o As Outer
    Set o = New Outer()
    Console.WriteLine(o.collection(42))
End Sub
"#.trim(),
    );
    assert_eq!(output, vec!["Item 42"]);
}
