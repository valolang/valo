//! Regression coverage for the native semantic migration.
use valo_core::{parse_source, run_source, semantics};

#[test]
fn readonly_byref_parameters_allow_reads_and_reject_direct_writes() {
    let output = run_source(
        "Function Inspect(ByRef ReadOnly Value As Integer) As Integer\nReturn Value + 1\nEnd Function\nSub Main()\nDim X As Integer = 3\nConsole.WriteLine(Inspect(X))\nConsole.WriteLine(X)\nEnd Sub",
    ).unwrap();
    assert_eq!(output, ["4", "3"]);

    let error = run_source(
        "Function Invalid(ByRef ReadOnly Value As Integer) As Integer\nValue = 9\nReturn Value\nEnd Function\nSub Main()\nDim X As Integer = 3\nConsole.WriteLine(Invalid(X))\nEnd Sub",
    ).unwrap_err();
    assert!(error.message.contains("ByRef ReadOnly"));

    let error = run_source(
        "Sub Mutate(ByRef Value As Integer)\nValue = 9\nEnd Sub\nSub Forward(ByRef ReadOnly Value As Integer)\nMutate(Value)\nEnd Sub\nSub Main()\nDim X As Integer = 3\nForward(X)\nEnd Sub",
    )
    .unwrap_err();
    assert!(error.message.contains("mutable ByRef"));
}

#[test]
fn declarations_without_type_or_initializer_are_rejected() {
    for source in [
        "Sub Main()\nDim Missing\nEnd Sub",
        "Sub Main()\nStatic Missing\nEnd Sub",
        "Sub Main()\nDim Typed As Integer, Missing\nEnd Sub",
        "Sub Main()\nStatic Typed As Integer, Missing\nEnd Sub",
        "Dim Missing\nSub Main()\nEnd Sub",
        "Dim Typed As Integer, Missing\nSub Main()\nEnd Sub",
    ] {
        let program = parse_source(source).unwrap();
        let diagnostic = semantics::validate(&program).expect_err(source);
        assert!(
            diagnostic
                .message
                .contains("cannot infer the type of 'Missing'"),
            "{source}: {diagnostic:?}"
        );
        assert!(diagnostic.span.is_some());
        assert!(diagnostic.helps.iter().any(
            |help| help.contains("Dim Missing As Integer") && help.contains("Dim Missing = 0")
        ));
    }
}

#[test]
fn inference_and_explicit_default_initialization_remain_supported() {
    let output = run_source(
        r#"
Dim ModuleCount = 42
Sub Main()
    Dim Count = ModuleCount
    Dim Name = "Valo"
    Dim Zero As Integer
    Static Total = 5
    Console.WriteLine(TypeName(Count))
    Console.WriteLine(TypeName(Name))
    Console.WriteLine(Zero)
    Console.WriteLine(Total)
End Sub
"#,
    )
    .unwrap();
    assert_eq!(output, ["Integer", "String", "0", "5"]);
}

#[test]
fn shared_as_clauses_preserve_vbnet_grouping() {
    let output = run_source(
        r#"
Dim First, Second As Integer
Sub Main()
    Dim A, B As Integer, C, D As Long
    Static E, F As String
    Console.WriteLine(TypeName(First))
    Console.WriteLine(TypeName(Second))
    Console.WriteLine(TypeName(A))
    Console.WriteLine(TypeName(B))
    Console.WriteLine(TypeName(C))
    Console.WriteLine(TypeName(D))
    Console.WriteLine(TypeName(E))
    Console.WriteLine(TypeName(F))
End Sub
"#,
    )
    .unwrap();
    assert_eq!(
        output,
        [
            "Integer", "Integer", "Integer", "Integer", "Long", "Long", "String", "String"
        ]
    );
    let error = parse_source("Sub Main()\nDim A, B As Integer = 1\nEnd Sub").unwrap_err();
    assert!(
        error
            .message
            .contains("initialize each variable separately")
    );
}

#[test]
fn omitted_types_remain_inference_holes_in_the_ast() {
    let program = parse_source("Dim Count = 10\nDim Missing\n").unwrap();
    assert!(program.module_vars.iter().all(|var| var.ty.is_none()));
}

#[test]
fn inferred_tuple_values_keep_structural_types_when_copied() {
    let output = run_source(
        r#"
Sub Main()
    Dim Original = (Count := 3, Text := "Valo")
    Dim Copy = Original
    Console.WriteLine(Copy.Count)
    Console.WriteLine(Copy.Text)
End Sub
"#,
    )
    .unwrap();
    assert_eq!(output, ["3", "Valo"]);
}

#[test]
fn legacy_module_options_are_rejected() {
    for directive in ["Option Base 0", "Option Base 1", "Option Private Module"] {
        let error = parse_source(&format!("{directive}\nSub Main()\nEnd Sub")).unwrap_err();
        assert!(error.message.contains("has been removed"));
        assert!(!error.helps.is_empty());
    }
}

#[test]
fn omitted_array_bounds_are_zero_for_declared_resized_and_variadic_arrays() {
    let output = run_source(
        r#"
Sub Inspect(ParamArray Values() As Variant)
    Console.WriteLine(LBound(Values))
    Console.WriteLine(UBound(Values))
End Sub
Sub Main()
    Dim Fixed(2) As Integer
    Dim Dynamic() As Integer
    ReDim Dynamic(2)
    Dynamic(0) = 7
    ReDim Preserve Dynamic(3)
    Console.WriteLine(LBound(Fixed))
    Console.WriteLine(LBound(Dynamic))
    Console.WriteLine(Dynamic(0))
    Inspect(1, 2)
End Sub
"#,
    )
    .unwrap();
    assert_eq!(output, ["0", "0", "7", "0", "1"]);
}

#[test]
fn exported_metadata_and_legacy_properties_are_rejected() {
    for source in [
        "Attribute VB_Name = \"Module1\"",
        "VERSION 1.0 CLASS\nBEGIN\nEND",
        "Class C\nAttribute VB_PredeclaredId = True\nEnd Class",
        "Sub Main()\nAttribute Main.VB_UserMemId = 0\nEnd Sub",
        "Class C\nProperty Get Value() As Integer\nEnd Property\nEnd Class",
        "Class C\nProperty Let Value(V As Integer)\nEnd Property\nEnd Class",
        "Class C\nProperty Set Value(V As Object)\nEnd Property\nEnd Class",
        "Interface I\nProperty Get Value() As Integer\nEnd Interface",
    ] {
        assert!(
            parse_source(source).is_err(),
            "accepted legacy source: {source}"
        );
    }
}

#[test]
fn modern_interface_and_module_properties_work() {
    let output = run_source(
        r#"
Interface ICounter
    Property Value As Integer
End Interface
Class Counter
    Implements ICounter
    Public Property Value As Integer Implements ICounter.Value
End Class
Module Program
    Public Property Total As Integer = 7
    Sub Main()
        Dim Counter As New Counter
        Counter.Value = Program.Total
        Program.Total += 1
        Console.WriteLine(Counter.Value)
        Console.WriteLine(Program.Total)
    End Sub
End Module
"#,
    )
    .unwrap();
    assert_eq!(output, ["7", "8"]);
}

#[test]
fn property_modifiers_do_not_silently_apply_to_functions() {
    for source in [
        "ReadOnly Function F() As Integer\nReturn 1\nEnd Function",
        "Module M\nWriteOnly Function F() As Integer\nReturn 1\nEnd Function\nEnd Module",
        "Interface I\nReadOnly Sub F()\nEnd Interface",
        "ReadOnly WriteOnly Property Value As Integer",
    ] {
        assert!(parse_source(source).is_err(), "accepted {source}");
    }
}
