use super::helpers::{run_source, source_error};

#[test]
fn for_each_object_native_iterator_returning_array() {
    let source = r#"
        Class Words
            Private values As Variant

            Public Constructor()
                values = Array("Valo", "is", "modern")
            End Constructor

            Public Iterator Items() As Variant
                Return values
            End Iterator
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
fn for_each_object_native_iterator_returning_variant_array() {
    let source = r#"
        Class Numbers
            Public Iterator Items() As Variant
                Items = Array(1, 2, 3)
            End Iterator
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
            Public Iterator One() As Variant
                Return Array(1)
            End Iterator

            Public Iterator Two() As Variant
                Return Array(2)
            End Iterator
        End Class
    "#;
    assert!(source_error(source).contains("multiple Iterator members"));
}

#[test]
fn iterator_returning_non_enumerable_is_rejected() {
    let source = r#"
        Class BadList
            Public Iterator Items() As Variant
                Return 42
            End Iterator
        End Class

        Sub Main()
            Dim list As New BadList
            Dim item As Variant
            For Each item In list
                Console.WriteLine(item)
            Next item
        End Sub
    "#;
    assert!(source_error(source).contains("did not return an enumerable value"));
}

#[test]
fn for_each_object_new_enum_property_returning_array() {
    let source = r#"
        Class List
            Private items As Variant

            Public Constructor()
                items = Array("a", "b")
            End Constructor

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
