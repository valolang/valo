//! The native direction must preserve the working VB.NET surface.
use valo_core::{TypeName, parse_source, run_source};

#[test]
fn primitive_aliases_have_fixed_widths() {
    for (alias, native, bits) in [
        ("Byte", "UInt8", 8),
        ("Short", "Int16", 16),
        ("Integer", "Int32", 32),
        ("Long", "Int64", 64),
        ("UInteger", "UInt32", 32),
        ("ULong", "UInt64", 64),
        ("Single", "Float32", 32),
        ("Double", "Float64", 64),
    ] {
        let ty = TypeName::from_primitive_name(alias).unwrap();
        assert_eq!(ty, TypeName::from_primitive_name(native).unwrap());
        assert_eq!(ty.numeric_bits(), Some(bits));
        let program = parse_source(&format!("Dim Value As {alias}\n")).unwrap();
        assert_eq!(program.module_vars[0].ty, Some(ty));
    }
    assert_eq!(
        TypeName::from_primitive_name("bOoL"),
        Some(TypeName::Boolean)
    );
}

#[test]
fn integer_long_conversions_and_inference_use_modern_widths() {
    let output = run_source(
        r#"
Module Program
    Sub Main()
        Dim Count As Integer = 100000
        Dim Size As Long = 5000000000
        Dim Inferred = 10
        Console.WriteLine(Count)
        Console.WriteLine(Size)
        Console.WriteLine(TypeName(Inferred))
        Console.WriteLine(CInt(100000))
        Console.WriteLine(CLng(5000000000))
    End Sub
End Module
"#,
    )
    .unwrap();
    assert_eq!(
        output,
        ["100000", "5000000000", "Integer", "100000", "5000000000"]
    );
}

#[test]
fn structures_copy_and_byref_modifies_only_the_borrowed_value() {
    let output = run_source(
        r#"
Structure Vec3
    Public X As Float32
    Public Y As Float32
    Public Z As Float32
End Structure

Sub Translate(ByRef Position As Vec3, ByVal Delta As Vec3)
    Position.X += Delta.X
End Sub

Sub Main()
    Dim A As Vec3
    A.X = 1
    Dim B As Vec3 = A
    Dim Delta As Vec3
    Delta.X = 10
    Translate(B, Delta)
    Console.WriteLine(A.X)
    Console.WriteLine(B.X)
End Sub
"#,
    )
    .unwrap();
    assert_eq!(output, ["1", "11"]);
}

#[test]
fn automation_is_not_a_language_intrinsic() {
    for expression in [
        "CreateObject(\"Excel.Application\")",
        "GetObject(\"workbook.xlsx\")",
        "DoEvents()",
    ] {
        assert!(
            run_source(&format!("Sub Main()\nDim Value = {expression}\nEnd Sub\n")).is_err(),
            "{expression}"
        );
    }
    // Removed intrinsic names remain available to ordinary user code.
    assert_eq!(run_source("Function CreateObject() As Integer\nReturn 7\nEnd Function\nSub Main()\nConsole.WriteLine(CreateObject())\nEnd Sub").unwrap(), ["7"]);
}

#[test]
fn ptrsafe_is_rejected_but_declare_is_preserved() {
    assert!(parse_source("Declare PtrSafe Function f Lib \"c\" () As Integer").is_err());
    assert!(parse_source("Declare Function f Lib \"c\" CDecl () As Int32").is_ok());
    assert!(run_source("Sub Main()\nDim Address As LongPtr\nEnd Sub").is_err());
}

#[test]
fn exported_modules_are_not_loadable() {
    for extension in ["bas", "cls"] {
        let (diagnostic, _) = valo_core::load_project(format!("removed.{extension}")).unwrap_err();
        assert!(
            diagnostic
                .to_string()
                .contains("must use the .valo extension")
        );
    }
}

#[test]
fn set_assignment_is_removed_without_removing_property_setters() {
    assert!(parse_source("Sub Main()\nDim Value As Object\nSet Value = Nothing\nEnd Sub").is_err());
    assert_eq!(
        run_source(
            r#"
Class Counter
    Private Stored As Integer
    Public Property Value As Integer
        Get
            Return Stored
        End Get
        Set(NewValue As Integer)
            Stored = NewValue
        End Set
    End Property
End Class
Sub Main()
    Dim Item As New Counter()
    Item.Value = 12
    Console.WriteLine(Item.Value)
End Sub
"#
        )
        .unwrap(),
        ["12"]
    );
}

#[test]
fn readonly_borrows_and_unsafe_pointers_are_not_silently_emulated() {
    for source in [
        "Sub Inspect(ByRef ReadOnly Value As Integer)\nEnd Sub",
        "Unsafe Sub WriteMemory()\nEnd Sub",
        "Sub Main()\nDim P As Pointer(Of Byte)\nEnd Sub",
    ] {
        assert!(run_source(source).is_err());
    }
}
