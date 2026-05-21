use crate::backend::interpreter::tests::helpers::*;

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
fn unary_numeric_literals_support_vba_suffixes_and_scientific_notation() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(-10#)
    Console.WriteLine(-10!)
    Console.WriteLine(-10&)
    Console.WriteLine(-10^)
    Console.WriteLine(-10@)
    Console.WriteLine(-1.5)
    Console.WriteLine(-.5)
    Console.WriteLine(-1E+3)
    Console.WriteLine(-2.5E-4)
    Console.WriteLine(+10#)
    Console.WriteLine(+1.5)
    Console.WriteLine(+.5)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "-10", "-10", "-10", "-10", "-10.0000", "-1.5", "-0.5", "-1000", "-0.00025", "10",
            "1.5", "0.5"
        ]
    );
}

#[test]
fn unary_numeric_operators_work_on_expressions_and_calls() {
    let output = run_source(
        r#"
Function Rand() As Double
    Rand = 1.25
End Function

Sub Main()
    Dim x As Double
    Dim y As Double
    x = 2.5
    y = 1.5
    Console.WriteLine(-(x + y))
    Console.WriteLine(-Rand())
    Console.WriteLine(--1)
    Console.WriteLine(+-1)
End Sub
"#,
    );

    assert_eq!(output, vec!["-4", "-1.25", "1", "-1"]);
}

#[test]
fn unary_numeric_precedence_matches_vba_power_behavior() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(-2 ^ 2)
    Console.WriteLine((-2) ^ 2)
    Console.WriteLine(2 ^ -2)
End Sub
"#,
    );

    assert_eq!(output, vec!["-4", "4", "0.25"]);
}

#[test]
fn unary_numeric_constants_and_radix_literals_fold() {
    let output = run_source(
        r#"
Const NEG As Long = -(5 + 2)
Const HEX As Long = -&H1
Const OCT As Long = -&O10

Sub Main()
    Console.WriteLine(NEG)
    Console.WriteLine(HEX)
    Console.WriteLine(OCT)
End Sub
"#,
    );

    assert_eq!(output, vec!["-7", "-1", "-8"]);
}

#[test]
fn unary_minus_rejects_non_numeric_values() {
    let diagnostic = source_diagnostic(
        r#"
Sub Main()
    Console.WriteLine(-"hello")
End Sub
"#,
    );

    assert_eq!(diagnostic.code.0, "V1100");
    assert!(diagnostic.message.contains("requires a numeric expression"));
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
