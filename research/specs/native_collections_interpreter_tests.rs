//! Future interpreter behavior specs for native Valo.Collections support.
//!
//! These are intentionally parked outside `core/src/backend/interpreter/tests.rs`
//! until the parser, semantic resolver, and runtime implement Rust-backed
//! `Valo.Collections.List(Of T)` and `Dictionary(Of K, V)` end to end.

use crate::run_source;

#[test]
fn list_works() {
    let source = r#"
Import Valo.Collections

Sub Main()
    Dim l As New List(Of String)()
    l.Add("Hello")
    l.Add("World")
    Console.WriteLine(l.Count)
    Console.WriteLine(l(0))
    Console.WriteLine(l(1))
End Sub
"#;
    let output = run_source(source).unwrap();
    assert_eq!(output, vec!["2", "Hello", "World"]);
}

#[test]
fn dictionary_works() {
    let source = r#"
Import Valo.Collections

Sub Main()
    Dim d As New Dictionary(Of String, Integer)()
    d.Add("A", 1)
    d.Add("B", 2)
    Console.WriteLine(d.Count)
    Console.WriteLine(d("A"))
    Console.WriteLine(d("B"))
    Console.WriteLine(d.Exists("A"))
    Console.WriteLine(d.ContainsKey("C"))
End Sub
"#;
    let output = run_source(source).unwrap();
    assert_eq!(output, vec!["2", "1", "2", "True", "False"]);
}

#[test]
fn collection_iteration_works() {
    let source = r#"
Import Valo.Collections

Sub Main()
    Dim l As New List(Of Integer)()
    l.Add(10)
    l.Add(20)
    Dim total As Integer
    Dim item As Variant
    For Each item In l
        total = total + item
    Next
    Console.WriteLine(total)

    Dim d As New Dictionary(Of String, String)()
    d.Add("K1", "V1")
    d.Add("K2", "V2")
    Dim keys As String
    For Each item In d
        keys = keys & item & " "
    Next
    Console.WriteLine(keys)
    Console.WriteLine("Done")
End Sub
"#;
    let output = run_source(source).unwrap();
    assert_eq!(output, vec!["30", "K1 K2 ", "Done"]);
}

#[test]
fn collection_type_safety_fails() {
    let source = r#"
Import Valo.Collections

Sub Main()
    Dim l As New List(Of Integer)()
    l.Add("Not an integer")
End Sub
"#;
    let result = run_source(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message
            .contains("Cannot assign String value to Integer variable")
            || err.message.contains("Type mismatch")
    );
}

#[test]
fn collection_unimported_fails() {
    let source = r#"
Sub Main()
    Dim l As New List(Of Integer)()
End Sub
"#;
    let result = run_source(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("is not defined"));
}
