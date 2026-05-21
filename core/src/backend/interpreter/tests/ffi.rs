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
fn strptr_requires_variable_to_avoid_temporary_pointer() {
    let diagnostic = source_diagnostic(
        r#"
Sub Main()
    Console.WriteLine(StrPtr("temporary"))
End Sub
"#,
    );

    assert_eq!(diagnostic.code.0, "V0001");
    assert!(
        diagnostic
            .message
            .contains("StrPtr requires a string variable")
    );
}
