use valo_core::backend::interpreter::run;

#[test]
fn test_lset_rset_strings() {
    let source = r#"
        Sub Main()
            Dim s As String
            s = "12345"
            LSet s = "abc"
            Console.WriteLine("L1:" & s)
            LSet s = "abcdefg"
            Console.WriteLine("L2:" & s)
            
            s = "12345"
            RSet s = "abc"
            Console.WriteLine("R1:" & s)
            RSet s = "abcdefg"
            Console.WriteLine("R2:" & s)
            
            Dim e As String
            LSet e = "test"
            Console.WriteLine("E1:" & e)
        End Sub
    "#;
    let program = valo_core::frontend::parser::Parser::parse_source(
        source,
        valo_core::runtime::FileId::default(),
    )
    .expect("Parse failed");
    valo_core::frontend::semantics::validate(&program).expect("Validation failed");
    let output = run(&program).expect("Run failed");
    assert_eq!(output[0], "L1:abc  ");
    assert_eq!(output[1], "L2:abcde");
    assert_eq!(output[2], "R1:  abc");
    assert_eq!(output[3], "R2:abcde");
    assert_eq!(output[4], "E1:");
}
