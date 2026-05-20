use crate::interpreter::run;
use crate::parser::Parser;
use crate::semantics::validate;

#[test]
fn test_numeric_ranges_and_overflow() {
    let source = "
        Sub Main()
            Dim b As Byte
            b = 255
            Console.WriteLine(b)
            
            Dim i As Integer
            i = 32767
            Console.WriteLine(i)
            
            Dim l As Long
            l = 2147483647
            Console.WriteLine(l)
            
            Dim i64 As Int64
            i64 = 9223372036854775807
            Console.WriteLine(i64)
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(
        output,
        vec!["255", "32767", "2147483647", "9223372036854775807"]
    );
}

#[test]
fn test_mixed_arithmetic() {
    let source = "
        Sub Main()
            Dim i As Integer
            Dim d As Double
            i = 10
            d = 2.5
            Console.WriteLine(i + d)
            Console.WriteLine(i * d)
            
            Dim s As Single
            s = 1.5
            Console.WriteLine(i + s)
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["12.5", "25", "11.5"]);
}

#[test]
fn test_conversions() {
    let source = "
        Sub Main()
            Dim v As Variant
            v = 123.456
            Console.WriteLine(CByte(v))
            Console.WriteLine(CInt(v))
            Console.WriteLine(CLng(v))
            Console.WriteLine(CDbl(v))
            
            Dim d As Date
            d = CDate(46152.0) ' Example serial date
            Console.WriteLine(TypeName(d))
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["123", "123", "123", "123.456", "Date"]);
}

#[test]
fn test_byte_array() {
    let source = "
        Sub Main()
            Dim data() As Byte
            ReDim data(0 To 1)
            data(0) = 65
            data(1) = 66
            Console.WriteLine(data(0))
            Console.WriteLine(data(1))
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["65", "66"]);
}

#[test]
fn byte_square_bracket_array_syntax_is_rejected() {
    let source = "
        Sub Main()
            Dim data As Byte[]
        End Sub
    ";
    let error = Parser::parse_source(source).unwrap_err().to_string();
    assert!(error.contains("Square-bracket array type syntax is not supported"));
}

#[test]
fn test_unsigned_types() {
    let source = "
        Sub Main()
            Dim u32 As UInt32
            u32 = 4294967295
            Console.WriteLine(u32)
            
            Dim u64 As UInt64
            u64 = 18446744073709551615
            Console.WriteLine(u64)
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["4294967295", "18446744073709551615"]);
}

#[test]
fn test_ptr_foundation() {
    let source = "
        Sub Main()
            Dim p As Ptr
            p = 0
            Console.WriteLine(TypeName(p))
            
            Dim f As FuncPtr
            f = 0
            Console.WriteLine(TypeName(f))
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["Ptr", "FuncPtr"]);
}
