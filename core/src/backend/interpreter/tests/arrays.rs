use crate::backend::interpreter::run;
use crate::frontend::parser::Parser;
use crate::frontend::semantics::validate;

#[test]
fn test_multidimensional_array() {
    let source = "
        Sub Main()
            Dim matrix(1 To 2, 0 To 1) As Integer
            matrix(1, 0) = 10
            matrix(1, 1) = 20
            matrix(2, 0) = 30
            matrix(2, 1) = 40
            Console.WriteLine(matrix(1, 0))
            Console.WriteLine(matrix(1, 1))
            Console.WriteLine(matrix(2, 0))
            Console.WriteLine(matrix(2, 1))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["10", "20", "30", "40"]);
}

#[test]
fn test_redim_preserve_multidimensional() {
    let source = "
        Sub Main()
            Dim m() As Integer
            ReDim m(1 To 2, 1 To 2)
            m(1, 1) = 1
            m(2, 2) = 4
            ReDim Preserve m(1 To 2, 1 To 3)
            Console.WriteLine(m(1, 1))
            Console.WriteLine(m(2, 2))
            Console.WriteLine(m(2, 3))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["1", "4", "0"]);
}

#[test]
fn test_array_builtin() {
    let source = "
        Sub Main()
            Dim v As Variant
            v = Array(1, \"hello\", True)
            Console.WriteLine(v(0))
            Console.WriteLine(v(1))
            Console.WriteLine(v(2))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["1", "hello", "True"]);
}

#[test]
fn test_split_join() {
    let source = "
        Sub Main()
            Dim s As String
            s = \"a,b,c\"
            Dim parts As Variant
            parts = Split(s, \",\")
            Console.WriteLine(parts(0))
            Console.WriteLine(parts(1))
            Console.WriteLine(parts(2))
            Console.WriteLine(Join(parts, \"-\"))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["a", "b", "c", "a-b-c"]);
}

#[test]
fn test_filter() {
    let source = "
        Sub Main()
            Dim fruits As Variant
            fruits = Array(\"apple\", \"banana\", \"cherry\", \"date\")
            Dim result As Variant
            result = Filter(fruits, \"a\")
            ' apple and banana contain \"a\"
            Dim f As Variant
            For Each f In result
                Console.WriteLine(f)
            Next
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["apple", "banana", "date"]);
}

#[test]
fn test_lbound_ubound_multidimensional() {
    let source = "
        Sub Main()
            Dim a(1 To 10, 5 To 15) As Integer
            Console.WriteLine(LBound(a, 1))
            Console.WriteLine(UBound(a, 1))
            Console.WriteLine(LBound(a, 2))
            Console.WriteLine(UBound(a, 2))
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["1", "10", "5", "15"]);
}

#[test]
fn test_for_each_multidimensional() {
    let source = "
        Sub Main()
            Dim m(0 To 1, 0 To 1) As Integer
            m(0, 0) = 1
            m(1, 0) = 2
            m(0, 1) = 3
            m(1, 1) = 4
            Dim x As Variant
            ' Traversal order should be column-major: (0,0), (1,0), (0,1), (1,1)
            For Each x In m
                Console.WriteLine(x)
            Next
        End Sub
    ";
    let program = Parser::parse_source(source, crate::runtime::FileId::default()).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["1", "2", "3", "4"]);
}
