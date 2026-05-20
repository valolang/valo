use crate::interpreter::tests::helpers::*;

#[test]
fn vba_compatibility_builtins_and_literals_work() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Variant
    value = Null
    Console.WriteLine(IsNull(value))
    Console.WriteLine(IsError(value))
    Console.WriteLine(VarType(Empty))
    Console.WriteLine(TypeName(Null))
    Console.WriteLine(IIf(True, "yes", "no"))
    Console.WriteLine(CStr(12))
    Console.WriteLine(StrComp("abc", "ABC", 1))
    Console.WriteLine(Sgn(-10))
    Console.WriteLine(Int(42))
    Console.WriteLine(7 \ 2)
End Sub
"#,
    );

    assert_eq!(
        output,
        vec![
            "True", "False", "0", "Null", "yes", "12", "0", "-1", "42", "3"
        ]
    );
}

#[test]
fn assigns_and_reads_members() {
    let output = run_source(
        r#"
Type User
    Name As String
    Age As Integer
End Type

Sub Main()
    Dim user As User
    user.Name = "Valo"
    user.Age = 1
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "1"]);
}

#[test]
fn type_and_field_names_are_case_insensitive() {
    let output = run_source(
        r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As user
    USER.name = "Valo"
    Console.WriteLine(user.NAME)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo"]);
}

#[test]
fn byref_type_parameter_can_mutate_field() {
    let output = run_source(
        r#"
Type User
    Name As String
    Active As Boolean
End Type

Sub Activate(ByRef user As User)
    user.Active = True
End Sub

Sub Main()
    Dim user As User
    user.Name = "Valo"
    Activate(user)
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Active)
End Sub
"#,
    );

    assert_eq!(output, vec!["Valo", "True"]);
}

#[test]
fn type_record_assignment_remains_value_copy() {
    let output = run_source(
        r#"
Type User
    Name As String
End Type

Sub Main()
    Dim a As User
    Dim b As User
    a.Name = "Original"
    b = a
    b.Name = "Changed"
    Console.WriteLine(a.Name)
End Sub
"#,
    );

    assert_eq!(output, vec!["Original"]);
}

#[test]
fn case_colon_single_line_body_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 1
    Select Case value
        Case 1: Console.WriteLine("one")
        Case Else: Console.WriteLine("other")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["one"]);
}

#[test]
fn case_else_colon_single_line_body_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 9
    Select Case value
        Case 1: Console.WriteLine("one")
        Case Else: Console.WriteLine("other")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["other"]);
}

#[test]
fn case_range_with_colon_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 4
    Select Case value
        Case 1 To 5: Console.WriteLine("small")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["small"]);
}

#[test]
fn case_is_with_colon_works() {
    let output = run_source(
        r#"
Sub Main()
    Dim value As Integer
    value = 11
    Select Case value
        Case Is > 10: Console.WriteLine("large")
    End Select
End Sub
"#,
    );

    assert_eq!(output, vec!["large"]);
}

#[test]
fn colon_separates_statements_outside_case() {
    let output = run_source(
        r#"
Sub Main()
    Console.WriteLine("a"): Console.WriteLine("b")
End Sub
"#,
    );

    assert_eq!(output, vec!["a", "b"]);
}

#[test]
fn line_continuation_joins_physical_lines() {
    let output = run_source(
        r#"
Sub Main()
    Dim total As Integer
    total = 10 + _
        20 + _
        30
    Console.WriteLine("hello " & _
        "world")
    Console.WriteLine(total)
End Sub
"#,
    );

    assert_eq!(output, vec!["hello world", "60"]);
}

#[test]
fn parameters_default_to_byref() {
    let output = run_source(
        r#"
Sub Increment(value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment x
    Console.WriteLine(x)
End Sub
"#,
    );

    assert_eq!(output, vec!["11"]);
}

#[test]
fn named_arguments_work_with_optional_parameters_out_of_order() {
    let output = run_source(
        r#"
Sub Greet(Optional ByVal name As String = "Valo", Optional ByVal title As String = "Runtime")
    Console.WriteLine(title & " " & name)
End Sub

Sub Main()
    Greet title := "OnlyTitle"
    Greet name := "Ada"
End Sub
"#,
    );

    assert_eq!(output, vec!["OnlyTitle Valo", "Runtime Ada"]);
}

#[test]
fn ismissing_detects_omitted_optional_variant() {
    let output = run_source(
        r#"
Sub Greet(Optional ByVal name As Variant)
    If IsMissing(name) Then
        Console.WriteLine("missing")
    Else
        Console.WriteLine(name)
    End If
End Sub

Sub WithDefault(Optional ByVal name As Variant = "Valo")
    Console.WriteLine(IsMissing(name))
End Sub

Sub Main()
    Greet
    Greet "Ada"
    WithDefault
End Sub
"#,
    );

    assert_eq!(output, vec!["missing", "Ada", "False"]);
}

#[test]
fn conditional_compilation_selects_active_branches_and_ignores_inactive_code() {
    let output = run_source(
        r#"
#Const Enabled = True
#Const Version = 2
#Const Target = "web"
Sub Main()
#If Enabled Then
    Console.WriteLine("enabled")
#Else
    this is not valid Valo
#End If
#If Target = "native" Then
    Console.WriteLine("native")
#ElseIf Target = "web" Then
    Console.WriteLine("web")
#Else
    Console.WriteLine("unknown")
#End If
#If Version > 1 And Not (Target = "native") Then
    Console.WriteLine("expr")
#End If
End Sub
"#,
    );
    assert_eq!(output, vec!["enabled", "web", "expr"]);
}

#[test]
fn conditional_compilation_supports_nested_blocks_and_else_branch() {
    let output = run_source(
        r#"
#Const Outer = True
#Const Inner = False
Sub Main()
#If Outer Then
#If Inner Then
    Console.WriteLine("inner")
#Else
    Console.WriteLine("nested else")
#End If
#Else
    Console.WriteLine("outer else")
#End If
End Sub
"#,
    );
    assert_eq!(output, vec!["nested else"]);
}

#[test]
fn conditional_compilation_builtin_valo_and_vba_constants_work() {
    let output = run_source(
        r#"
Sub Main()
#If VALO Then
    Console.WriteLine("VALO")
#Else
    invalid inactive Valo
#End If
#If Valo Then
    Console.WriteLine("Valo")
#End If
#If valoruntime Then
    Console.WriteLine("ValoRuntime")
#End If
#If VBA7 Then
    Console.WriteLine("VBA7")
#End If
#If VBA6 Then
    invalid inactive VBA6
#Else
    Console.WriteLine("not VBA6")
#End If
End Sub
"#,
    );

    assert_eq!(
        output,
        vec!["VALO", "Valo", "ValoRuntime", "VBA7", "not VBA6"]
    );
}

#[test]
fn conditional_compilation_builtin_build_platform_and_arch_constants_work() {
    let output = run_source(
        r#"
Sub Main()
#If Debug Or Release Then
    Console.WriteLine("build")
#Else
    invalid inactive build
#End If
#If Windows Or Linux Or MacOS Or Android Or IOS Or FreeBSD Or OpenBSD Or NetBSD Or DragonFly Or Solaris Or Illumos Or Haiku Or Wasm Or Unix Then
    Console.WriteLine("platform")
#Else
    invalid inactive platform
#End If
#If X86 Or X64 Or Arm Or Arm64 Or Armv7 Or RiscV32 Or RiscV64 Or Wasm32 Or Wasm64 Or S390x Or PowerPC Or PowerPC64 Or Mips Or Mips64 Or LoongArch64 Then
    Console.WriteLine("arch")
#Else
    invalid inactive arch
#End If
End Sub
"#,
    );

    assert_eq!(output, vec!["build", "platform", "arch"]);
}

#[test]
fn conditional_compilation_builtin_vba_platform_aliases_are_available() {
    let output = run_source(
        r#"
Sub Main()
#If Win32 Or Win64 Or Mac Or Mac64 Or Not (Win32 Or Win64 Or Mac Or Mac64) Then
    Console.WriteLine("aliases")
#Else
    invalid inactive aliases
#End If
End Sub
"#,
    );

    assert_eq!(output, vec!["aliases"]);
}

#[test]
fn conditional_compilation_user_const_can_override_builtin_constant() {
    let output = run_source(
        r#"
#Const Valo = False
Sub Main()
#If Valo Then
    invalid inactive override
#Else
    Console.WriteLine("override")
#End If
End Sub
"#,
    );

    assert_eq!(output, vec!["override"]);
}
