use crate::interpreter::tests::helpers::*;

#[test]
fn enum_auto_increment_after_explicit_value() {
    let output = run_source(
        r#"
Enum Numbers
    Zero
    Five = 5
    Six
End Enum

Sub Main()
    Console.WriteLine(Zero)
    Console.WriteLine(Six)
End Sub
"#,
    );

    assert_eq!(output, vec!["0", "6"]);
}

#[test]
fn rejects_duplicate_and_unknown_enum_members() {
    let duplicate = source_error(
        r#"
Enum Bad
    One
    one
End Enum

Sub Main()
End Sub
"#,
    );
    assert!(duplicate.contains("Enum member 'one' is already declared"));

    let unknown = source_error(
        r#"
Enum Bad
    Two = One + 1
End Enum

Sub Main()
End Sub
"#,
    );
    assert!(unknown.contains("Enum member 'One' is not defined"));
}

