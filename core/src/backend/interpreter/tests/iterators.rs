use super::helpers::{run_source, source_error};

#[test]
fn for_each_object_native_iterator_function_yielding() {
    let source = r#"
        Class Words
            Public Iterator Function Items() As Variant
                Yield "Valo"
                Yield "is"
                Yield "modern"
            End Function
        End Class

        Sub Main()
            Dim words As New Words
            Dim item As Variant
            For Each item In words
                Console.WriteLine(item)
            Next item
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["Valo", "is", "modern"]);
}

#[test]
fn for_each_object_native_iterator_property_yielding() {
    let source = r#"
        Class Numbers
            Public Iterator Property Get Items() As Variant
                Yield 1
                Yield 2
                Yield 3
            End Property
        End Class

        Sub Main()
            Dim numbers As New Numbers
            Dim item As Variant
            For Each item In numbers
                Console.WriteLine(item)
            Next item
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["1", "2", "3"]);
}

#[test]
fn duplicate_native_iterator_is_rejected() {
    let source = r#"
        Class BadList
            Public Iterator Function One() As Variant
                Yield 1
            End Function

            Public Iterator Function Two() As Variant
                Yield 2
            End Function
        End Class
    "#;
    assert!(source_error(source).contains("multiple default Iterator members"));
}

#[test]
fn yield_outside_iterator_is_rejected() {
    let source = r#"
        Sub Main()
            Yield 1
        End Sub
    "#;
    assert!(source_error(source).contains("Yield is only allowed inside Iterator functions"));
}

#[test]
fn iterator_function_without_yield_is_rejected() {
    let source = r#"
        Class Bad
            Public Iterator Function Items() As Variant
            End Function
        End Class
        Sub Main()
        End Sub
    "#;
    assert!(source_error(source).contains("must contain at least one Yield statement"));
}

#[test]
fn iterator_function_with_byref_param_is_rejected() {
    let source = r#"
        Class Bad
            Public Iterator Function Items(ByRef x As Integer) As Variant
                Yield x
            End Function
        End Class
        Sub Main()
        End Sub
    "#;
    assert!(source_error(source).contains("cannot have ByRef parameters"));
}

#[test]
fn return_inside_iterator_is_rejected() {
    let source = r#"
        Class Bad
            Public Iterator Function Items() As Variant
                Return Array(1)
            End Function
        End Class
        Sub Main()
        End Sub
    "#;
    assert!(source_error(source).contains("Return is not allowed inside Iterator"));
}

#[test]
fn old_iterator_block_is_rejected() {
    let source = r#"
        Class Bad
            Public Iterator Items() As Variant
                Return Array(1)
            End Iterator
        End Class

        Sub Main()
        End Sub
    "#;
    assert!(source_error(source).contains("Expected Function or Property after Iterator"));
}

#[test]
fn end_iterator_is_rejected() {
    let source = r#"
        Class Bad
            Public Iterator Function Items() As Variant
                Yield 1
            End Iterator
        End Class

        Sub Main()
        End Sub
    "#;
    assert!(source_error(source).contains("Expected statement"));
}

#[test]
fn for_each_object_new_enum_property_returning_array() {
    let source = r#"
        Class List
            Private items As Variant

            Public Sub New()
                items = Array("a", "b")
            End Sub

            Public Property Get _NewEnum() As Variant
            Attribute _NewEnum.VB_UserMemId = -4
                _NewEnum = items
            End Property
        End Class

        Sub Main()
            Dim list As New List
            Dim item As Variant
            For Each item In list
                Console.WriteLine(item)
            Next item
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["a", "b"]);
}

#[test]
fn for_each_object_new_enum_function_returning_array() {
    let source = r#"
        Class List
            Public Function _NewEnum() As Variant
            Attribute _NewEnum.VB_UserMemId = -4
                Return Array("x", "y")
            End Function
        End Class

        Sub Main()
            Dim list As New List
            Dim item As Variant
            For Each item In list
                Debug.Print item
            Next item
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["x", "y"]);
}

#[test]
fn new_enum_attribute_is_case_insensitive_and_exported_style() {
    let source = r#"
        Class List
            Public Property Get nEwEnUm() As Variant
            Attribute nEwEnUm.VB_UserMemId = -4
                nEwEnUm = Array("case", "ok")
            End Property
        End Class

        Sub Main()
            Dim list As New List
            Dim item As Variant
            For Each item In list
                Console.WriteLine(item)
            Next item
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["case", "ok"]);
}

#[test]
fn default_property_user_mem_id_zero_still_works() {
    let source = r#"
        Class Box
            Public Property Get Value() As String
            Attribute Value.VB_UserMemId = 0
                Value = "default"
            End Property
        End Class

        Sub Main()
            Dim box As New Box
            Console.WriteLine(box)
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["default"]);
}

#[test]
fn missing_iterator_or_new_enum_has_readable_diagnostic() {
    let source = r#"
        Class Plain
        End Class

        Sub Main()
            Dim plain As New Plain
            Dim item As Variant
            For Each item In plain
                Console.WriteLine(item)
            Next item
        End Sub
    "#;
    assert!(
        source_error(source).contains("define an Iterator or a VB_UserMemId = -4 _NewEnum member")
    );
}

#[test]
fn iterator_function_with_parameters_called_explicitly() {
    let source = r#"
        Class Generator
            Public Iterator Function Range(ByVal count As Integer) As Variant
                Dim i As Integer
                For i = 1 To count
                    Yield i
                Next i
            End Function
        End Class

        Sub Main()
            Dim gen As New Generator
            Dim n As Variant
            For Each n In gen.Range(3)
                Console.WriteLine(n)
            Next n
        End Sub
    "#;
    assert_eq!(run_source(source), vec!["1", "2", "3"]);
}
