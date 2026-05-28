use valo_core::backend::interpreter::run;
use valo_core::frontend::parser::Parser;
use valo_core::frontend::semantics::validate;
use valo_core::runtime::FileId;

fn exec(source: &str) -> Vec<String> {
    let program = Parser::parse_source(source, FileId::default()).expect("Parse failed");
    validate(&program).expect("Validation failed");
    run(&program).expect("Run failed")
}

#[test]
fn test_lset_rset() {
    let source = r#"
        Sub Main()
            Dim s As String
            s = "12345"
            LSet s = "abc"
            Console.WriteLine(s)
            RSet s = "abc"
            Console.WriteLine(s)
            s = "12345"
            LSet s = "abcdefg"
            Console.WriteLine(s)
        End Sub
    "#;
    let output = exec(source);
    assert_eq!(output[0], "abc  ");
    assert_eq!(output[1], "  abc");
    assert_eq!(output[2], "abcde");
}

#[test]
fn test_like_advanced() {
    let source = r#"
        Sub Main()
            Console.WriteLine("1:" & ("abc" Like "a[a-z]c"))
            Console.WriteLine("2:" & ("a5c" Like "a[0-9]c"))
            Console.WriteLine("3:" & ("abc" Like "a[!0-9]c"))
            Console.WriteLine("4:" & ("a*c" Like "a[*]c"))
        End Sub
    "#;
    let output = exec(source);
    assert_eq!(output[0], "1:True");
    assert_eq!(output[1], "2:True");
    assert_eq!(output[2], "3:True");
    assert_eq!(output[3], "4:True");
}

#[test]
fn test_builtins_parity() {
    let source = r#"
        Sub Main()
            Console.WriteLine("1:" & IsNumeric(" 123.4 "))
            Console.WriteLine("2:" & IsDate("2026-05-27"))
            Dim p As Variant
            p = Split("a b c", " ")
            Console.WriteLine("3:" & Join(p, ","))
        End Sub
    "#;
    let output = exec(source);
    assert_eq!(output[0], "1:True");
    assert_eq!(output[1], "2:True");
    assert_eq!(output[2], "3:a,b,c");
}
