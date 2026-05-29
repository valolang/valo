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

#[test]
fn test_collection() {
    let source = r#"
        Sub Main()
            Dim c As New Collection
            c.Add "item1", "key1"
            c.Add "item2"
            Console.WriteLine("C1:" & c.Count)
            Console.WriteLine("C2:" & c("key1"))
            Console.WriteLine("C3:" & c(2))
            
            Dim s As String
            s = ""
            Dim v As Variant
            For Each v In c
                s = s & v & ","
            Next v
            Console.WriteLine("C4:" & s)
            
            c.Remove "key1"
            Console.WriteLine("C5:" & c.Count)
            Console.WriteLine("C6:" & c(1))
        End Sub
    "#;
    let output = exec(source);
    assert_eq!(output[0], "C1:2");
    assert_eq!(output[1], "C2:item1");
    assert_eq!(output[2], "C3:item2");
    assert_eq!(output[3], "C4:item1,item2,");
    assert_eq!(output[4], "C5:1");
    assert_eq!(output[5], "C6:item2");
}
