use crate::interpreter::tests::helpers::*;

#[test]
fn evaluates_and_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(True And True)
    Console.WriteLine(True And False)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False"]);
}

#[test]
fn evaluates_or_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(False Or True)
    Console.WriteLine(False Or False)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False"]);
}

#[test]
fn evaluates_not_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(Not True)
    Console.WriteLine(Not False)
End Sub
"#,
    );

    assert_eq!(output, vec!["False", "True"]);
}

#[test]
fn evaluates_mod_result() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(10 Mod 3)
End Sub
"#,
    );

    assert_eq!(output, vec!["1"]);
}

#[test]
fn logical_operator_precedence() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(True Or False And False)
    Console.WriteLine(Not False And False)
    Console.WriteLine(Not (False And False))
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "False", "True"]);
}

#[test]
fn like_operator_supports_vba_wildcards_and_option_compare() {
    let output = run_source(
        r#"
Option Compare Text

Sub Main()
    Console.WriteLine("Ada" Like "A*")
    Console.WriteLine("Ada" Like "A?a")
    Console.WriteLine("A7" Like "A#")
    Console.WriteLine("B" Like "[ABC]")
    Console.WriteLine("D" Like "[!ABC]")
    Console.WriteLine("ada" Like "A*")
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "True", "True", "True", "True", "True"]);
}

