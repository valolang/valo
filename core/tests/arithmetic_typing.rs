use valo_core::runtime::{
    FileId, Span,
    compare::RuntimeOptionCompare,
    ops::{RuntimeBinaryOp, eval_binary},
};
use valo_core::{TypeName, Value, run_source};

#[test]
fn arithmetic_inference_agrees_with_execution() {
    let output = run_source(
        r#"
Function Pick(Value As Integer) As String
    Return "Int32"
End Function
Function Pick(Value As Long) As String
    Return "Int64"
End Function
Sub Main()
    Dim I As Integer = 3
    Dim L As Long = 3
    Dim F As Single = 3
    Dim Sum = I + I
    Console.WriteLine(TypeName(Sum))
    Console.WriteLine(Pick(I + I))
    Console.WriteLine(TypeName(I + L))
    Console.WriteLine(TypeName(F * F))
    Console.WriteLine(TypeName(I / I))
    Console.WriteLine(TypeName(I ^ I))
    Console.WriteLine(TypeName(I \ I))
    Console.WriteLine(5.5 Mod 2.0)
End Sub
"#,
    )
    .unwrap();
    assert_eq!(
        output,
        [
            "Integer", "Int32", "Long", "Single", "Double", "Double", "Integer", "1.5"
        ]
    );
}

#[test]
fn fixed_width_arithmetic_wraps_without_promoting_or_panicking() {
    for (a, op, b, expected, ty) in [
        (
            Value::Int32(i32::MAX),
            RuntimeBinaryOp::Add,
            Value::Int32(1),
            Value::Int32(i32::MIN),
            TypeName::Int32,
        ),
        (
            Value::UInt64(u64::MAX),
            RuntimeBinaryOp::Add,
            Value::UInt64(1),
            Value::UInt64(0),
            TypeName::UInt64,
        ),
        (
            Value::Int64(i64::MIN),
            RuntimeBinaryOp::IntegerDivide,
            Value::Int64(-1),
            Value::Int64(i64::MIN),
            TypeName::Int64,
        ),
        (
            Value::Int32(i32::MIN),
            RuntimeBinaryOp::Modulo,
            Value::Int32(-1),
            Value::Int32(0),
            TypeName::Int32,
        ),
    ] {
        let result = eval_binary(
            a,
            op,
            b,
            RuntimeOptionCompare::Binary,
            Span::empty(FileId::default()),
        )
        .unwrap();
        assert_eq!(result, expected);
        assert_eq!(result.type_name(), ty);
    }
}
