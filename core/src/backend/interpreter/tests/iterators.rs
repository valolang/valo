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
            Public ReadOnly Iterator Property Items() As Variant
                Get
                Yield 1
                Yield 2
                Yield 3
                End Get
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
fn iterator_enumerates_stored_array() {
    let source = r#"
        Class List
            Private items As Variant

            Public Sub New()
                items = Array("a", "b")
            End Sub

            Public Iterator Function Enumerate() As Variant

                Dim Element As Variant

                For Each Element In items

                    Yield Element

                Next

            End Function
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
fn iterator_yields_values() {
    let source = r#"
        Class List
            Public Iterator Function Items() As Variant
                Yield "x"
                Yield "y"
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
fn iterator_enumerates_inline_array() {
    let source = r#"
        Class List
            Public Iterator Function Items() As Variant
                Dim Element As Variant
                For Each Element In Array("case", "ok")
                    Yield Element
                Next
            End Function
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
fn default_property_keyword_selects_default_member() {
    let source = r#"
        Class Box
            Public ReadOnly Default Property Value() As String
                Get
                Value = "default"
                End Get
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
fn missing_iterator_has_readable_diagnostic() {
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
    assert!(source_error(source).contains("define an Iterator member"));
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
