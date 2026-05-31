use super::helpers::{run_source, source_diagnostic};
use crate::runtime::ffi_platform::*;

#[test]
fn declare_function_calls_libc_strlen_with_byval_string() {
    let source = format!(
        r#"
Private Declare PtrSafe Function lstrlen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long

Sub Main()
    Console.WriteLine(lstrlen("Valo"))
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["4"]);
}

#[test]
fn declare_function_is_callable_as_statement_with_parentheses() {
    let source = format!(
        r#"
Private Declare PtrSafe Function strlen Lib "{}" CDecl (ByVal value As String) As Long

Sub Main()
    strlen("Valo")
    Console.WriteLine("ok")
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["ok"]);
}

#[test]
fn declare_function_is_callable_with_call_keyword() {
    let source = format!(
        r#"
Public Declare PtrSafe Function strlen Lib "{}" CDecl (ByVal value As String) As Long

Sub Main()
    Call strlen("Valo")
    Console.WriteLine("ok")
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["ok"]);
}

#[test]
fn declare_function_is_callable_as_bare_statement() {
    let source = format!(
        r#"
Private Declare PtrSafe Function strlen Lib "{}" CDecl (ByVal value As String) As Long

Sub Main()
    strlen "Valo"
    Console.WriteLine("ok")
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["ok"]);
}

#[test]
fn declare_function_statement_call_validates_arguments() {
    let diagnostic = source_diagnostic(&format!(
        r#"
Private Declare PtrSafe Function strlen Lib "{}" CDecl (ByVal value As String) As Long

Sub Main()
    strlen()
End Sub
"#,
        platform_libc()
    ));

    assert_eq!(diagnostic.code.0, "V0001");
    assert!(diagnostic.message.contains("Function 'strlen' expects"));
}

#[test]
#[cfg(unix)]
fn declare_sub_calls_libc_srand() {
    let source = format!(
        r#"
Private Declare PtrSafe Sub Seed Lib "{}" Alias "srand" CDecl (ByVal value As Long)

Sub Main()
    Seed 1
    Console.WriteLine("ok")
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["ok"]);
}

#[test]
fn declare_function_calls_libm_with_double_argument_and_return() {
    let source = format!(
        r#"
Private Declare PtrSafe Function NativeCos Lib "{}" Alias "{}" CDecl (ByVal value As Double) As Double

Sub Main()
    Console.WriteLine(NativeCos(0#))
End Sub
"#,
        platform_libm(),
        platform_test_math_symbol()
    );

    assert_eq!(run_source(&source), vec!["1"]);
}

#[test]
fn declare_function_accepts_unary_numeric_argument() {
    let source = format!(
        r#"
Private Declare PtrSafe Function NativeCos Lib "{}" Alias "{}" CDecl (ByVal value As Double) As Double

Sub Main()
    Console.WriteLine(NativeCos(-1#) < 1#)
End Sub
"#,
        platform_libm(),
        platform_test_math_symbol()
    );

    assert_eq!(run_source(&source), vec!["True"]);
}

#[test]
fn ffi_vector_math_stress_completes_with_structure_array_mutation() {
    let source = format!(
        r#"
Private Declare PtrSafe Function NativeCos Lib "{}" Alias "{}" CDecl (ByVal value As Double) As Double
Private Declare PtrSafe Function NativeSin Lib "{}" Alias "sin" CDecl (ByVal value As Double) As Double
Private Declare PtrSafe Function NativeSqrt Lib "{}" Alias "sqrt" CDecl (ByVal value As Double) As Double

Structure Vec3
    Public X As Double
    Public Y As Double
    Public Z As Double
End Structure

Function MakeVec(ByVal i As Integer) As Vec3
    Dim v As Vec3
    v.X = (i Mod 17) - 8.5
    v.Y = (i Mod 23) - 11.5
    v.Z = (i Mod 31) - 15.5
    Return v
End Function

Function Length(ByVal v As Vec3) As Double
    Length = NativeSqrt(v.X * v.X + v.Y * v.Y + v.Z * v.Z)
End Function

Function Normalize(ByVal v As Vec3) As Vec3
    Dim result As Vec3
    Dim len As Double
    len = Length(v)
    If len = 0# Then
        Return v
    End If
    result.X = v.X / len
    result.Y = v.Y / len
    result.Z = v.Z / len
    Return result
End Function

Function RotateY(ByVal v As Vec3, ByVal angle As Double) As Vec3
    Dim result As Vec3
    Dim c As Double
    Dim s As Double
    c = NativeCos(angle)
    s = NativeSin(angle)
    result.X = v.X * c + v.Z * s
    result.Y = v.Y
    result.Z = -v.X * s + v.Z * c
    Return result
End Function

Sub Main()
    Const Count As Integer = 120
    Dim points() As Vec3
    Dim i As Integer
    Dim pass As Integer
    Dim angle As Double
    Dim total As Double
    Dim avg As Double

    ReDim points(0 To Count - 1)
    For i = 0 To Count - 1
        points(i) = MakeVec(i)
    Next i

    For pass = 1 To 4
        angle = pass * 0.0174532925199433#
        For i = 0 To Count - 1
            points(i) = Normalize(points(i))
            points(i) = RotateY(points(i), angle)
        Next i
    Next pass

    For i = 0 To Count - 1
        total = total + Length(points(i))
    Next i
    avg = total / Count
    Console.WriteLine(avg > 0.999# And avg < 1.001#)
End Sub
"#,
        platform_libm(),
        platform_test_math_symbol(),
        platform_libm(),
        platform_libm()
    );

    assert_eq!(run_source(&source), vec!["True"]);
}

#[test]
fn declare_alias_uses_local_name_for_semantics_and_native_symbol_for_lookup() {
    let source = format!(
        r#"
Private Declare PtrSafe Function MyLen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long

Sub Main()
    Console.WriteLine(MyLen("Valo"))
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["4"]);
}

#[test]
fn declare_byref_numeric_argument_is_written_back() {
    let source = format!(
        r#"
Private Declare PtrSafe Function NativeTime Lib "{}" Alias "time" CDecl (value As LongPtr) As LongPtr

Sub Main()
    Dim value As LongPtr
    Console.WriteLine(NativeTime(value) <> 0)
    Console.WriteLine(value <> 0)
End Sub
"#,
        platform_libc()
    );

    assert_eq!(run_source(&source), vec!["True", "True"]);
}

#[test]
#[cfg(windows)]
fn declare_windows_get_current_process_id() {
    let source = r#"
Private Declare PtrSafe Function GetCurrentProcessId Lib "kernel32" () As Long

Sub Main()
    Console.WriteLine(GetCurrentProcessId() <> 0)
End Sub
"#;
    assert_eq!(run_source(source), vec!["True"]);
}

#[test]
#[cfg(windows)]
fn declare_windows_get_tick_count_64() {
    let source = r#"
Private Declare PtrSafe Function GetTickCount64 Lib "kernel32" () As LongLong

Sub Main()
    ' Just verify it calls correctly and returns a value
    Dim val As LongLong
    val = GetTickCount64()
    Console.WriteLine("Done")
End Sub
"#;
    assert_eq!(run_source(source), vec!["Done"]);
}

#[test]
#[cfg(windows)]
fn declare_windows_lstrlen_a() {
    let source = r#"
Private Declare PtrSafe Function lstrlenA Lib "kernel32" (ByVal lpString As String) As Long

Sub Main()
    Dim length As Long
    length = lstrlenA("Hello")
    Console.WriteLine(length)
End Sub
"#;
    assert_eq!(run_source(source), vec!["5"]);
}

#[test]
#[cfg(unix)]
fn declare_unix_getpid() {
    let source = format!(
        r#"
Private Declare PtrSafe Function getpid Lib "{}" () As Long

Sub Main()
    Console.WriteLine(getpid() <> 0)
End Sub
"#,
        platform_libc()
    );
    assert_eq!(run_source(&source), vec!["True"]);
}

#[test]
fn missing_native_library_reports_v3001() {
    let diagnostic = source_diagnostic(
        r#"
Private Declare PtrSafe Function Nope Lib "valo_missing_native_library_for_test" () As Long

Sub Main()
    Console.WriteLine(Nope())
End Sub
"#,
    );

    assert_eq!(diagnostic.code.0, "V3001");
    assert!(
        diagnostic
            .message
            .contains("native library `valo_missing_native_library_for_test` could not be loaded")
    );
}

#[test]
fn missing_native_symbol_reports_v3002() {
    let source = format!(
        r#"
Private Declare PtrSafe Function Nope Lib "{}" Alias "valo_missing_symbol_for_test" CDecl () As Long

Sub Main()
    Console.WriteLine(Nope())
End Sub
"#,
        platform_libc()
    );
    let diagnostic = source_diagnostic(&source);

    assert_eq!(diagnostic.code.0, "V3002");
    assert!(
        diagnostic
            .message
            .contains("symbol `valo_missing_symbol_for_test` was not found")
    );
}

#[test]
fn unsupported_byref_string_reports_v3003() {
    let source = format!(
        r#"
Private Declare PtrSafe Function lstrlen Lib "{}" Alias "strlen" CDecl (value As String) As Long

Sub Main()
    Dim value As String
    value = "Valo"
    Console.WriteLine(lstrlen(value))
End Sub
"#,
        platform_libc()
    );
    let diagnostic = source_diagnostic(&source);

    assert_eq!(diagnostic.code.0, "V3003");
    assert!(diagnostic.message.contains("ByRef String buffers"));
}

#[test]
fn addressof_byval_numeric_callback_returns_stable_pointer() {
    let source = r#"
Function MyCallback(ByVal value As Long) As Long
    MyCallback = value + 1
End Function

Sub Main()
    Dim ptr As LongPtr
    ptr = AddressOf MyCallback
    Console.WriteLine(ptr <> 0)
End Sub
"#;

    assert_eq!(run_source(source), vec!["True"]);
}

#[test]
fn addressof_byref_callback_reports_v3003() {
    let diagnostic = source_diagnostic(
        r#"
Function MyCallback(value As Long) As Long
    MyCallback = value
End Function

Sub Main()
    Dim ptr As LongPtr
    ptr = AddressOf MyCallback
End Sub
"#,
    );

    assert_eq!(diagnostic.code.0, "V3003");
    assert!(
        diagnostic
            .message
            .contains("AddressOf callbacks currently require ByVal")
    );
}

#[test]
fn strptr_accepts_temporary_string_expressions() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine(StrPtr("temporary") <> 0)
    Console.WriteLine(StrPtr(CStr(123)) <> 0)
    Console.WriteLine(StrPtr(Left$("abc", 1)) <> 0)
End Sub
"#,
    );

    assert_eq!(output, vec!["True", "True", "True"]);
}

#[test]
#[cfg(windows)]
fn varptr_saved_pointer_writes_back_to_original_variable_after_native_call() {
    let output = run_source(
        r#"
Private Declare PtrSafe Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As LongPtr, ByVal Source As LongPtr, ByVal Length As LongPtr)

Sub Main()
    Dim source As LongLong
    Dim target As LongLong
    Dim pointer As LongPtr
    source = 123456
    pointer = VarPtr(target)
    RtlMoveMemory ByVal pointer, VarPtr(source), 8
    Console.WriteLine(target)
    Console.WriteLine(VarType(pointer))
End Sub
"#,
    );

    assert_eq!(output, vec!["123456", "20"]);
}
