use crate::interpreter::run;
use crate::parser::Parser;
use crate::semantics::validate;

#[test]
fn test_callbyname() {
    let source = "
        Class Target
            Public Value As Integer
            Public Sub SetValue(ByVal v As Integer)
                Value = v
            End Sub
            Public Function GetValue() As Integer
                GetValue = Value
            End Function
        End Class

        Sub Main()
            Dim obj As New Target
            ' VbMethod = 1
            CallByName obj, \"SetValue\", 1, 42
            Console.WriteLine(obj.Value)
            
            ' VbGet = 2
            Console.WriteLine(CallByName(obj, \"GetValue\", 1))
            Console.WriteLine(CallByName(obj, \"Value\", 2))
            
            ' VbLet = 4
            CallByName obj, \"Value\", 4, 99
            Console.WriteLine(obj.Value)
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["42", "42", "42", "99"]);
}

#[test]
fn test_vba_constants() {
    let source = "
        Sub Main()
            Console.WriteLine(vbBinaryCompare)
            Console.WriteLine(vbTextCompare)
            Console.WriteLine(vbString)
            Console.WriteLine(vbArray)
            Console.WriteLine(VbMethod)
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["0", "1", "8", "8192", "1"]);
}

#[test]
fn test_random() {
    let source = "
        Sub Main()
            Randomize 123
            Dim r1 As Double
            r1 = Rnd()
            Randomize 123
            Dim r2 As Double
            r2 = Rnd()
            Console.WriteLine(r1)
            Console.WriteLine(r2)
            ' Deterministic seeding
            If r1 = r2 Then
                Console.WriteLine(\"matched\")
            End If
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output[2], "matched");
}

#[test]
fn test_vba_namespace() {
    let source = "
        Sub Main()
            Dim parts As Variant
            parts = VBA.Split(\"a,b,c\", \",\")
            Console.WriteLine(VBA.Join(parts, \"-\"))
            Console.WriteLine(VBA.TypeName(123))
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["a-b-c", "Integer"]);
}

#[test]
fn test_isempty() {
    let source = "
        Sub Main()
            Dim v As Variant
            Console.WriteLine(IsEmpty(v))
            v = 1
            Console.WriteLine(IsEmpty(v))
            v = Empty
            Console.WriteLine(IsEmpty(v))
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["True", "False", "True"]);
}

#[test]
fn test_return_modernization() {
    let source = "
        Function Test(ByVal x As Integer) As Integer
            If x > 10 Then
                Return x * 2
            End If
            Test = x + 1
        End Function

        Sub Main()
            Console.WriteLine(Test(15))
            Console.WriteLine(Test(5))
        End Sub
    ";
    let program = Parser::parse_source(source).unwrap();
    validate(&program).unwrap();
    let output = run(&program).unwrap();
    assert_eq!(output, vec!["30", "6"]);
}
