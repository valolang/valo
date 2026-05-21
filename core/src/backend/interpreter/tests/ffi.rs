use super::helpers::{run_source, source_diagnostic};

#[cfg(unix)]
fn libc_name() -> &'static str {
    #[cfg(target_os = "android")]
    {
        "libc.so"
    }
    #[cfg(all(unix, not(target_os = "android")))]
    {
        "libc.so.6"
    }
}

#[cfg(unix)]
fn libm_name() -> &'static str {
    #[cfg(target_os = "android")]
    {
        "libm.so"
    }
    #[cfg(all(unix, not(target_os = "android")))]
    {
        "libm.so.6"
    }
}

#[test]
#[cfg(unix)]
fn declare_function_calls_libc_strlen_with_byval_string() {
    let source = format!(
        r#"
Private Declare PtrSafe Function lstrlen Lib "{}" Alias "strlen" CDecl (ByVal value As String) As Long

Sub Main()
    Console.WriteLine(lstrlen("Valo"))
End Sub
"#,
        libc_name()
    );

    assert_eq!(run_source(&source), vec!["4"]);
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
        libc_name()
    );

    assert_eq!(run_source(&source), vec!["ok"]);
}

#[test]
#[cfg(unix)]
fn declare_function_calls_libm_with_double_argument_and_return() {
    let source = format!(
        r#"
Private Declare PtrSafe Function NativeCos Lib "{}" Alias "cos" CDecl (ByVal value As Double) As Double

Sub Main()
    Console.WriteLine(NativeCos(0#))
End Sub
"#,
        libm_name()
    );

    assert_eq!(run_source(&source), vec!["1"]);
}

#[test]
#[cfg(unix)]
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
        libc_name()
    );

    assert_eq!(run_source(&source), vec!["True", "True"]);
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
#[cfg(unix)]
fn missing_native_symbol_reports_v3002() {
    let source = format!(
        r#"
Private Declare PtrSafe Function Nope Lib "{}" Alias "valo_missing_symbol_for_test" CDecl () As Long

Sub Main()
    Console.WriteLine(Nope())
End Sub
"#,
        libc_name()
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
#[cfg(unix)]
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
        libc_name()
    );
    let diagnostic = source_diagnostic(&source);

    assert_eq!(diagnostic.code.0, "V3003");
    assert!(diagnostic.message.contains("ByRef String buffers"));
}
