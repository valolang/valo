use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

use valo_core::backend::llvm::{EmitKind, LlvmTools, NativeOptions, build, render_module};
use valo_core::mir::{ir, lower_module};
use valo_core::parse_source;
use valo_core::semantics::lower_function_body;

fn module(source: &str) -> ir::Module {
    let program = parse_source(source).unwrap();
    let entry = program
        .functions
        .iter()
        .position(|function| function.name.eq_ignore_ascii_case("main"));
    let bodies = (0..program.functions.len())
        .map(|index| lower_function_body(&program, index).unwrap())
        .collect::<Vec<_>>();
    let mut module = lower_module(&bodies).unwrap();
    module.entry = entry.map(valo_core::semantics::typed_hir::BodyFunctionId);
    module
}

fn tools() -> Option<LlvmTools> {
    match LlvmTools::discover() {
        Ok(tools) => Some(tools),
        Err(error) => {
            eprintln!("LLVM native test skipped: {error}");
            None
        }
    }
}

fn output(name: &str) -> PathBuf {
    let extension = if cfg!(windows) { "exe" } else { "out" };
    std::env::temp_dir().join(format!(
        "valo-native-{}-{name}.{extension}",
        std::process::id()
    ))
}

fn execute(source: &str, expected: i32) {
    let Some(tools) = tools() else {
        return;
    };
    let module = module(source);
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hash);
    let path = output(&format!("execution-{:016x}", hash.finish()));
    let artifact = build(
        &module,
        &tools,
        &NativeOptions {
            output: path.clone(),
            kind: EmitKind::Executable,
            optimize: false,
        },
    )
    .unwrap();
    assert_eq!(artifact.path, path);
    let status = Command::new(&path)
        .env("VALO_RUNTIME_ASSERT_CLEAN", "1")
        .status()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(status.code(), Some(expected));
}

#[test]
fn native_string_identity_copy_return_and_unicode_length() {
    execute(
        "Function Identity(Value As String) As String\nReturn Value\nEnd Function\nFunction Main() As Integer\nDim A As String = \"Valo 🦊\"\nDim B As String = Identity(A)\nIf A = B Then\nIf Len(B) = 6 Then\nReturn 0\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_string_default_empty_and_escaped_quotes() {
    execute(
        "Function Main() As Integer\nDim S As String\nIf S <> \"\" Then\nReturn 1\nEnd If\nS = \"A\"\"B\"\nIf S = \"A\"\"B\" Then\nReturn 0\nEnd If\nReturn 2\nEnd Function",
        0,
    );
}

#[test]
fn native_string_concat_replacement_self_assignment_and_branch_cleanup() {
    execute(
        "Function Main() As Integer\nDim A As String = \"Va\"\nDim B As String = \"lo\"\nDim C As String = A & B\nC = C & \"!\"\nC = C\nIf C <> \"Valo!\" Then\nReturn 1\nEnd If\nIf Len(C) = 5 Then\nReturn 0\nEnd If\nReturn 2\nEnd Function",
        0,
    );
}

#[test]
fn native_string_byref_and_readonly_keep_ownership() {
    execute(
        "Function Change(ByRef S As String) As Integer\nS = S & \"!\"\nReturn 0\nEnd Function\nFunction Length(ByRef ReadOnly S As String) As Integer\nReturn Len(S)\nEnd Function\nFunction Main() As Integer\nDim S As String = \"Hello\"\nDim Result As Integer = Change(S)\nIf Length(S) = 6 Then\nReturn Result\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_string_interpolation_formats_supported_numeric_holes() {
    execute(
        "Function Main() As Integer\nDim Average As Double = 2.5\nDim Count As Integer = 7\nDim S As String = $\"Valo {Average:0.0}ms {Count}\"\nIf S = \"Valo 2.5ms 7\" Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_option_compare_text_is_rejected_precisely() {
    let Some(tools) = tools() else {
        return;
    };
    let module = module(
        "Option Compare Text\nFunction Main() As Integer\nIf \"A\" = \"a\" Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
    );
    let error = render_module(&module, &tools.target).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Option Compare Text string comparisons"),
        "{error}"
    );
}

#[test]
fn native_option_compare_binary_is_content_based_and_case_sensitive() {
    execute(
        "Option Compare Binary\nFunction Main() As Integer\nIf \"A\" = \"a\" Then\nReturn 1\nEnd If\nIf \"A\" < \"a\" Then\nReturn 0\nEnd If\nReturn 2\nEnd Function",
        0,
    );
}

#[test]
fn native_string_drop_runs_on_return_and_after_finally() {
    execute(
        "Function Main() As Integer\nDim S As String = \"A\" & \"B\"\nTry\nReturn Len(S)\nFinally\nS = S & \"C\"\nEnd Try\nEnd Function",
        2,
    );
    execute(
        "Function Main() As Integer\nDim S As String = \"A\"\nTry\nS = S & \"B\"\nFinally\nS = S & \"C\"\nEnd Try\nIf S = \"ABC\" Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
    execute(
        "Function Main() As Integer\nTry\nReturn 0\nFinally\nDim S As String = \"A\" & \"B\"\nEnd Try\nEnd Function",
        0,
    );
}

#[test]
fn native_string_inner_scopes_and_loop_iterations_drop() {
    execute(
        "Function Main() As Integer\nDim I As Integer = 0\nWhile I < 3\nDim S As String = \"X\" & \"Y\"\nIf Len(S) <> 2 Then\nReturn 1\nEnd If\nI += 1\nWend\nReturn 0\nEnd Function",
        0,
    );
}

#[test]
fn native_string_early_return_skips_later_uninitialized_drop() {
    execute(
        "Function Main() As Integer\nDim Choice As Boolean = True\nIf Choice Then\nReturn 0\nEnd If\nDim S As String = \"A\" & \"B\"\nReturn Len(S)\nEnd Function",
        0,
    );
}

#[test]
fn native_managed_structure_clone_replacement_and_drop() {
    execute(
        "Structure Person\nPublic Name As String\nPublic Age As Integer\nEnd Structure\nFunction Main() As Integer\nDim A As Person\nA.Name = \"Kane\" & \"ki\"\nA.Age = 18\nDim B As Person = A\nA.Name = A.Name & \"!\"\nIf B.Name <> \"Kaneki\" Then\nReturn 1\nEnd If\nIf A.Name <> \"Kaneki!\" Then\nReturn 2\nEnd If\nA = B\nA = A\nIf A.Name = B.Name Then\nIf A.Age = 18 Then\nReturn 0\nEnd If\nEnd If\nReturn 3\nEnd Function",
        0,
    );
}

#[test]
fn native_nested_managed_structure_byval_return_and_readonly() {
    execute(
        "Structure Profile\nPublic Name As String\nEnd Structure\nStructure Player\nPublic Profile As Profile\nPublic Score As Integer\nEnd Structure\nFunction Make() As Player\nDim P As Player\nP.Profile.Name = \"Valo\" & \"!\"\nP.Score = 42\nReturn P\nEnd Function\nFunction Check(ByRef ReadOnly P As Player) As Integer\nIf P.Profile.Name = \"Valo!\" Then\nReturn P.Score\nEnd If\nReturn 1\nEnd Function\nFunction Main() As Integer\nDim P As Player = Make()\nDim Q As Player = P\nIf Check(Q) = 42 Then\nReturn 0\nEnd If\nReturn 2\nEnd Function",
        0,
    );
}

#[test]
fn native_class_references_alias_and_release_string_fields() {
    execute(
        "Class Person\nPublic Name As String\nPublic Age As Integer\nEnd Class\nFunction Main() As Integer\nDim A As Person = New Person()\nA.Name = \"Kane\" & \"ki\"\nA.Age = 18\nDim B As Person = A\nB.Age = 19\nIf A.Age <> 19 Then\nReturn 1\nEnd If\nIf B.Name <> \"Kaneki\" Then\nReturn 2\nEnd If\nB = B\nIf A.Name = \"Kaneki\" Then\nReturn 0\nEnd If\nReturn 3\nEnd Function",
        0,
    );
}

#[test]
fn native_class_return_and_byval_keep_object_alive() {
    execute(
        "Class Person\nPublic Name As String\nEnd Class\nFunction Make() As Person\nDim P As Person = New Person()\nP.Name = \"Va\" & \"lo\"\nReturn P\nEnd Function\nFunction Check(P As Person) As Integer\nIf P.Name = \"Valo\" Then\nReturn 42\nEnd If\nReturn 1\nEnd Function\nFunction Main() As Integer\nDim P As Person = Make()\nIf Check(P) = 42 Then\nReturn 0\nEnd If\nReturn 2\nEnd Function",
        0,
    );
}

#[test]
fn native_as_new_class_and_reference_replacement_are_safe() {
    execute(
        "Class Box\nPublic Value As Integer\nEnd Class\nFunction Main() As Integer\nDim A As New Box()\nA.Value = 20\nDim B As New Box()\nB.Value = 22\nA = B\nA = A\nIf A.Value + B.Value = 44 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_class_nothing_and_identity_are_pointer_based() {
    execute(
        "Class Person\nEnd Class\nFunction Main() As Integer\nDim A As Person = Nothing\nIf A IsNot Nothing Then\nReturn 1\nEnd If\nA = New Person()\nDim B As Person = A\nDim C As Person = New Person()\nIf A Is B Then\nIf A IsNot C Then\nIf C IsNot Nothing Then\nReturn 0\nEnd If\nEnd If\nEnd If\nReturn 2\nEnd Function",
        0,
    );
}

#[test]
fn native_nested_class_fields_and_managed_structure_copy() {
    execute(
        "Class Person\nPublic Name As String\nEnd Class\nClass Holder\nPublic Person As Person\nEnd Class\nStructure Wrapper\nPublic Ref As Holder\nEnd Structure\nFunction Main() As Integer\nDim H As New Holder()\nH.Person = New Person()\nH.Person.Name = \"Va\" & \"lo\"\nDim W As Wrapper\nW.Ref = H\nDim C As Wrapper = W\nIf C.Ref.Person.Name = \"Valo\" Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_collection_stores_tagged_string_and_integer_values() {
    execute(
        "Function Main() As Integer\nDim Items As New Collection()\nItems.Add(\"Va\" & \"lo\")\nItems.Add(42)\nIf Items.Count <> 2 Then\nReturn 1\nEnd If\nDim Name As String = CType(Items.Item(1), String)\nDim Number As Integer = CType(Items.Item(2), Integer)\nItems.Remove(1)\nIf Items.Count <> 1 Then\nReturn 2\nEnd If\nIf Name = \"Valo\" Then\nIf Number = 42 Then\nReturn 0\nEnd If\nEnd If\nReturn 3\nEnd Function",
        0,
    );
}

#[test]
fn native_collection_retains_class_and_copies_value_structure() {
    execute(
        "Class Person\nPublic Age As Integer\nEnd Class\nStructure Point\nPublic X As Integer\nEnd Structure\nFunction Main() As Integer\nDim Items As New Collection()\nDim P As New Person()\nP.Age = 18\nDim V As Point\nV.X = 42\nItems.Add(P)\nItems.Add(V, , 1)\nP = Nothing\nDim Q As Person = CType(Items.Item(2), Person)\nDim W As Point = CType(Items.Item(1), Point)\nIf Q.Age = 18 Then\nIf W.X = 42 Then\nReturn 0\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_collection_foreach_snapshots_order_and_mutation() {
    execute(
        "Function Main() As Integer\nDim Items As New Collection()\nItems.Add(1)\nItems.Add(2)\nDim Item As Variant\nDim Total As Integer = 0\nFor Each Item In Items\nTotal += CType(Item, Integer)\nItems.Add(9)\nNext\nIf Total = 3 Then\nIf Items.Count = 4 Then\nReturn 0\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_collection_foreach_return_releases_snapshot_and_items() {
    execute(
        "Function Main() As Integer\nDim Items As New Collection()\nItems.Add(42)\nDim Item As Variant\nFor Each Item In Items\nReturn CType(Item, Integer)\nNext\nReturn 1\nEnd Function",
        42,
    );
}

#[test]
fn native_collection_foreach_empty_exit_and_continue_cleanup() {
    execute(
        "Function Main() As Integer\nDim Items As New Collection()\nDim Item As Variant\nDim Total As Integer = 0\nFor Each Item In Items\nTotal += 100\nNext\nItems.Add(1)\nItems.Add(2)\nItems.Add(3)\nFor Each Item In Items\nIf CType(Item, Integer) = 1 Then\nContinue For\nEnd If\nTotal += CType(Item, Integer)\nIf Total = 2 Then\nExit For\nEnd If\nNext\nReturn Total\nEnd Function",
        2,
    );
}

#[test]
fn managed_values_propagate_copy_and_drop_properties() {
    use valo_core::TypeName;
    use valo_core::frontend::semantics::type_properties::{CopyKind, KnownProperty, properties};
    let program = parse_source(
        "Class Person\nPublic Name As String\nEnd Class\nStructure Profile\nPublic Name As String\nEnd Structure\nStructure Player\nPublic Data As Profile\nPublic Owner As Person\nEnd Structure",
    ).unwrap();
    for ty in [
        TypeName::String,
        TypeName::Variant,
        TypeName::User("Collection".into()),
        TypeName::User("Person".into()),
        TypeName::User("Profile".into()),
        TypeName::User("Player".into()),
    ] {
        let found = properties(&program, &ty);
        assert_eq!(found.copy_kind(), CopyKind::Managed, "{ty:?}");
        assert_eq!(found.requires_drop, KnownProperty::Yes, "{ty:?}");
    }
    assert_eq!(
        properties(&program, &TypeName::Int32).copy_kind(),
        CopyKind::Trivial
    );
}

#[test]
fn native_collection_return_byval_and_temporary_class_lifetime() {
    execute(
        "Class Person\nPublic Age As Integer\nEnd Class\nFunction MakeItems() As Collection\nDim Items As New Collection()\nItems.Add(New Person())\nReturn Items\nEnd Function\nFunction CountItems(ByVal Items As Collection) As Integer\nReturn Items.Count\nEnd Function\nFunction Main() As Integer\nDim Items As Collection = MakeItems()\nIf CountItems(Items) = 1 Then\nDim PersonValue As Person = CType(Items.Item(1), Person)\nIf PersonValue.Age = 0 Then\nReturn 0\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_collection_byref_replacement_and_readonly_borrow() {
    execute(
        "Function ReplaceItems(ByRef Items As Collection) As Integer\nItems = New Collection()\nItems.Add(42)\nReturn 0\nEnd Function\nFunction ReadCount(ByRef ReadOnly Items As Collection) As Integer\nReturn Items.Count\nEnd Function\nFunction Main() As Integer\nDim Items As New Collection()\nItems.Add(1)\nItems.Add(2)\nDim Ignored As Integer = ReplaceItems(Items)\nIf ReadCount(Items) = 1 Then\nReturn CType(Items.Item(1), Integer)\nEnd If\nReturn 1\nEnd Function",
        42,
    );
}

#[test]
fn native_class_byref_replacement_and_readonly_field_access() {
    execute(
        "Class Person\nPublic Age As Integer\nEnd Class\nFunction ReplacePerson(ByRef P As Person) As Integer\nP = New Person()\nP.Age = 42\nReturn 0\nEnd Function\nFunction ReadAge(ByRef ReadOnly P As Person) As Integer\nReturn P.Age\nEnd Function\nFunction Main() As Integer\nDim P As New Person()\nP.Age = 10\nDim Ignored As Integer = ReplacePerson(P)\nReturn ReadAge(P)\nEnd Function",
        42,
    );
}

#[test]
fn native_collection_key_restriction_has_specific_hir_diagnostic() {
    let program = parse_source("Function Main() As Integer\nDim Items As New Collection()\nItems.Add(1, \"one\")\nReturn 0\nEnd Function").unwrap();
    let error = lower_function_body(&program, 0).unwrap_err();
    assert!(error.message.contains("Collection String keys"), "{error}");
}

#[test]
fn module_array_boundary_remains_explicit() {
    let array = parse_source("Private Depth(0 To 1) As Integer\nFunction Main() As Integer\nReturn Depth(0)\nEnd Function").unwrap();
    let error = lower_function_body(&array, 0).unwrap_err();
    assert!(
        error.message.contains("module-level array storage"),
        "{error}"
    );
}

#[test]
fn native_select_case_evaluates_selector_once_and_checks_cases_in_order() {
    let source = "Function NextValue(ByRef N As Integer) As Integer\nN = N + 1\nReturn N\nEnd Function\nFunction Main() As Integer\nDim N As Integer = 0\nSelect Case NextValue(N)\nCase 0\nReturn 1\nCase 2 To 4\nReturn 2\nCase Is > 4\nReturn 3\nCase 1, 5\nIf N = 1 Then\nReturn 0\nEnd If\nReturn 4\nCase Else\nReturn 5\nEnd Select\nEnd Function";
    execute(source, 0);
    let interpreted = source.replace("Function Main() As Integer", "Function Result() As Integer")
        + "\nSub Main()\nConsole.WriteLine(Result())\nEnd Sub";
    assert_eq!(valo_core::run_source(&interpreted).unwrap(), ["0"]);
}

#[test]
fn native_select_case_range_and_compare_execute() {
    execute(
        "Function Main() As Integer\nDim X As Integer = 7\nSelect Case X\nCase 0 To 6\nReturn 1\nCase Is > 6\nReturn 0\nCase Else\nReturn 2\nEnd Select\nEnd Function",
        0,
    );
}

#[test]
fn native_select_case_evaluates_both_range_bounds_even_when_lower_fails() {
    let source = "Function Tick(ByRef N As Integer) As Integer\nN = N + 1\nReturn N\nEnd Function\nFunction Main() As Integer\nDim N As Integer = 0\nSelect Case 0\nCase 2 To Tick(N)\nReturn 1\nCase Else\nReturn N - 1\nEnd Select\nEnd Function";
    execute(source, 0);
    let interpreted = source.replace("Function Main() As Integer", "Function Result() As Integer")
        + "\nSub Main()\nConsole.WriteLine(Result())\nEnd Sub";
    assert_eq!(valo_core::run_source(&interpreted).unwrap(), ["0"]);
}

#[test]
fn native_select_case_managed_selector_has_precise_boundary() {
    let source = "Function Main() As Integer\nSelect Case \"A\"\nCase \"A\"\nReturn 0\nCase Else\nReturn 1\nEnd Select\nEnd Function";
    let program = parse_source(source).unwrap();
    let error = lower_function_body(&program, 0).unwrap_err();
    assert!(
        error
            .message
            .contains("Select Case over this selector type"),
        "{error}"
    );
}

#[test]
fn native_select_case_preserves_loop_exit_and_continue_targets() {
    execute(
        "Function Main() As Integer\nDim I As Integer = 0\nWhile I < 5\nI = I + 1\nSelect Case I\nCase 1\nContinue While\nCase 3\nExit While\nCase Else\nI = I\nEnd Select\nWend\nReturn I - 3\nEnd Function",
        0,
    );
}

#[test]
fn managed_fixed_array_is_rejected_before_codegen() {
    let Some(tools) = tools() else { return };
    let program = module(
        "Function Main() As Integer\nDim Names(1) As String\nNames(0) = \"Valo\"\nReturn 0\nEnd Function",
    );
    let error = render_module(&program, &tools.target).unwrap_err();
    assert_eq!(error.stage, "native eligibility");
    assert!(error.message.contains("managed fixed arrays"), "{error}");
}

#[test]
fn native_class_with_user_termination_is_rejected_before_allocation() {
    let program = parse_source("Class Resource\nPublic Sub Terminate()\nEnd Sub\nEnd Class\nFunction Main() As Integer\nDim R As New Resource()\nReturn 0\nEnd Function").unwrap();
    let error = lower_function_body(&program, 0).unwrap_err();
    assert!(error.message.contains("Terminate finalizer"), "{error}");
}

#[test]
fn native_managed_tuple_clone_and_drop() {
    execute(
        "Function Main() As Integer\nDim A = (\"Va\" & \"lo\", 42)\nDim B = A\nIf B.Item1 = \"Valo\" Then\nIf B.Item2 = 42 Then\nReturn 0\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn mir_verifier_rejects_reuse_of_owned_string_temp() {
    let mut module = module(
        "Function Main() As Integer\nDim S As String = \"A\" & \"B\"\nIf S = \"AB\" Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
    );
    let function = &mut module.functions[0];
    let compare = function
        .blocks
        .iter_mut()
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| matches!(instruction.kind, ir::InstructionKind::StringCompare { .. }))
        .unwrap();
    if let ir::InstructionKind::StringCompare { left, right, .. } = &mut compare.kind {
        *right = *left;
    }
    let error = valo_core::mir::verify::verify(function).unwrap_err();
    assert!(error.contains("owned managed temp"), "{error}");
}

#[test]
fn interpreter_and_native_agree_on_supported_string_values() {
    let source = "Function Result() As Integer\nDim A As String = \"Va\"\nDim B As String = A & \"lo\"\nB = B\nIf A = \"Va\" Then\nIf B = \"Valo\" Then\nIf Len(\"á🦊\") = 2 Then\nReturn 0\nEnd If\nEnd If\nEnd If\nReturn 1\nEnd Function";
    let interpreted = format!("{source}\nSub Main()\nConsole.WriteLine(Result())\nEnd Sub\n");
    assert_eq!(valo_core::run_source(&interpreted).unwrap(), ["0"]);
    execute(&source.replace("Function Result()", "Function Main()"), 0);
}

#[test]
fn primitive_native_executable_returns_constant() {
    execute("Function Main() As Integer\nReturn 42\nEnd Function", 42);
}

#[test]
fn arithmetic_and_direct_call_execute_natively() {
    execute(
        "Function Add(A As Integer, B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Main() As Integer\nReturn Add(20, 22)\nEnd Function",
        42,
    );
}

#[test]
fn forward_call_uses_predeclared_function_signature() {
    execute(
        "Function Main() As Integer\nReturn Add(20, 22)\nEnd Function\nFunction Add(A As Integer, B As Integer) As Integer\nReturn A + B\nEnd Function",
        42,
    );
}

#[test]
fn branch_and_loop_execute_natively() {
    execute(
        "Function Max(A As Integer, B As Integer) As Integer\nIf A > B Then\nReturn A\nEnd If\nReturn B\nEnd Function\nFunction Main() As Integer\nDim Total As Integer = 0\nDim I As Integer = 0\nFor I = 1 To 4\nTotal += I\nNext\nReturn Max(Total, 9)\nEnd Function",
        10,
    );
}

#[test]
fn recursion_uses_declared_symbols_before_bodies() {
    execute(
        "Function Fact(N As Integer) As Integer\nIf N <= 1 Then\nReturn 1\nEnd If\nReturn N * Fact(N - 1)\nEnd Function\nFunction Main() As Integer\nReturn Fact(5)\nEnd Function",
        120,
    );
}

#[test]
fn signed_and_unsigned_comparisons_and_division_differ() {
    execute(
        "Function Main() As Integer\nDim A As Integer = 0 - 9\nDim B As UInteger = 9\nDim C As UInteger = 2\nIf A < 0 Then\nIf B > C Then\nReturn B \\ C\nEnd If\nEnd If\nReturn 1\nEnd Function",
        4,
    );
    execute(
        "Function Main() As Integer\nDim A As Integer = 0 - 9\nReturn A \\ 2\nEnd Function",
        -4,
    );
    let Some(tools) = tools() else {
        return;
    };
    let unsigned = module(
        "Function Main() As Integer\nDim A As UInteger = 9\nDim B As UInteger = 2\nIf A > B Then\nReturn A \\ B\nEnd If\nReturn 0\nEnd Function",
    );
    let ir = render_module(&unsigned, &tools.target).unwrap();
    assert!(ir.contains("udiv i32"), "{ir}");
    assert!(ir.contains("icmp ugt i32"), "{ir}");
}

#[test]
fn primitive_byref_and_readonly_addresses_work() {
    execute(
        "Function Change(ByRef X As Integer, ByRef ReadOnly Y As Integer) As Integer\nX += Y\nReturn X\nEnd Function\nFunction Main() As Integer\nDim A As Integer = 20\nDim B As Integer = 22\nReturn Change(A, B)\nEnd Function",
        42,
    );
}

#[test]
fn integer_to_float_and_float_widening_casts_work() {
    execute(
        "Function Main() As Integer\nDim A As Single = 3\nDim B As Double = A\nIf B = 3 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
    execute(
        "Function Main() As Integer\nDim A As Double = 1.5\nDim B As Double = 0.5\nIf A / B = 3.0 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn optimized_native_executable_runs() {
    let Some(tools) = tools() else {
        return;
    };
    let module =
        module("Function Main() As Integer\nDim X As Integer = 20\nReturn X + 22\nEnd Function");
    let path = output("optimized");
    build(
        &module,
        &tools,
        &NativeOptions {
            output: path.clone(),
            kind: EmitKind::Executable,
            optimize: true,
        },
    )
    .unwrap();
    assert_eq!(Command::new(&path).status().unwrap().code(), Some(42));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn finally_cleanup_cfg_preserves_return_payload() {
    execute(
        "Function Main() As Integer\nDim X As Integer = 1\nTry\nReturn X\nFinally\nX = 2\nEnd Try\nEnd Function",
        1,
    );
    execute(
        "Function Main() As Integer\nDim X As Integer = 0\nTry\nX = 1\nFinally\nX += 2\nEnd Try\nReturn X\nEnd Function",
        3,
    );
}

#[test]
fn while_do_exit_continue_and_negative_for_step_run_natively() {
    execute(
        "Function Main() As Integer\nDim I As Integer = 0\nDim Total As Integer = 0\nWhile I < 3\nI += 1\nIf I = 2 Then\nContinue While\nEnd If\nTotal += I\nWend\nDo While I < 5\nI += 1\nIf I = 5 Then\nExit Do\nEnd If\nTotal += I\nLoop\nFor I = 3 To 1 Step 0 - 1\nTotal += I\nNext\nReturn Total\nEnd Function",
        14,
    );
}

#[test]
fn unsupported_checked_narrowing_is_a_controlled_backend_error() {
    let Some(tools) = tools() else {
        return;
    };
    let program = module(
        "Function Main() As Integer\nDim X As Integer = 1000\nDim Y As Short = X\nReturn Y\nEnd Function",
    );
    let error = render_module(&program, &tools.target).unwrap_err();
    assert_eq!(error.stage, "native eligibility");
    assert!(error.message.contains("checked conversion"), "{error}");
}

#[test]
fn unresolved_layout_invalid_entry_and_native_drop_report_precise_errors() {
    use valo_core::frontend::semantics::type_properties::{KnownProperty, TypeProperties};
    let Some(tools) = tools() else {
        return;
    };
    let mut program = module("Function Main(A As Integer) As Integer\nReturn 0\nEnd Function");
    program.functions[0].locals[0].ty = valo_core::TypeName::User("Resource".into());
    let error = render_module(&program, &tools.target).unwrap_err();
    assert_eq!(error.stage, "native eligibility");
    assert!(error.message.contains("native value layout"));

    program.functions[0].locals[0].ty = valo_core::TypeName::Variant;
    let error = render_module(&program, &tools.target).unwrap_err();
    assert_eq!(error.stage, "native eligibility");
    assert!(error.message.contains("entry point must be"), "{error}");

    let mut program =
        module("Function Main() As Integer\nDim X As Integer = 1\nReturn X\nEnd Function");
    let function = &mut program.functions[0];
    function.locals[0].properties = TypeProperties {
        copy: KnownProperty::No,
        requires_drop: KnownProperty::Yes,
    };
    let place = ir::Place {
        root: ir::LocalId(0),
        projections: vec![],
        ty: valo_core::TypeName::Int32,
    };
    let span = function.span;
    let block = function
        .blocks
        .iter_mut()
        .find(|b| {
            matches!(
                b.terminator.as_ref().map(|t| &t.kind),
                Some(ir::TerminatorKind::Return(_))
            )
        })
        .unwrap();
    block.instructions.push(ir::Instruction {
        result: None,
        kind: ir::InstructionKind::Drop(place),
        span,
    });
    let error = render_module(&program, &tools.target).unwrap_err();
    assert_eq!(error.stage, "native eligibility");
    assert!(
        error.message.contains("no supported native destructor"),
        "{error}"
    );
}

#[test]
fn llvm_ir_is_verified_and_object_is_emitted() {
    let Some(tools) = tools() else {
        return;
    };
    let module = module("Function Main() As Integer\nReturn 7\nEnd Function");
    let ir = render_module(&module, &tools.target).unwrap();
    assert!(ir.contains("define i32 @valo_"));
    let ir_path = output("ir").with_extension("ll");
    build(
        &module,
        &tools,
        &NativeOptions {
            output: ir_path.clone(),
            kind: EmitKind::LlvmIr,
            optimize: false,
        },
    )
    .unwrap();
    assert!(
        std::fs::read_to_string(&ir_path)
            .unwrap()
            .contains("target datalayout")
    );
    let obj = output("ir").with_extension(if cfg!(windows) { "obj" } else { "o" });
    build(
        &module,
        &tools,
        &NativeOptions {
            output: obj.clone(),
            kind: EmitKind::Object,
            optimize: true,
        },
    )
    .unwrap();
    assert!(std::fs::metadata(&obj).unwrap().len() > 0);
    assert!(
        std::fs::read_to_string(&ir_path)
            .unwrap()
            .contains("define i32 @main")
    );
    std::fs::remove_file(ir_path).unwrap();
    std::fs::remove_file(obj).unwrap();
}

#[test]
fn native_vec2_readonly_borrow_and_field_places() {
    execute(
        "Structure Vec2\nPublic X As Single\nPublic Y As Single\nEnd Structure\nFunction LengthSquared(ByRef ReadOnly V As Vec2) As Single\nReturn V.X * V.X + V.Y * V.Y\nEnd Function\nFunction Main() As Integer\nDim V As Vec2\nV.X = 3\nV.Y = 4\nIf LengthSquared(V) = 25 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_nested_struct_copy_byval_and_return() {
    execute(
        "Structure Vec2\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nStructure Transform\nPublic Position As Vec2\nPublic Active As Boolean\nEnd Structure\nFunction Make() As Transform\nDim T As Transform\nT.Position.X = 20\nT.Position.Y = 22\nT.Active = True\nReturn T\nEnd Function\nFunction Score(T As Transform) As Integer\nReturn T.Position.X + T.Position.Y\nEnd Function\nFunction Main() As Integer\nDim A As Transform = Make()\nDim B As Transform = A\nB.Position.X = 1\nIf A.Active Then\nIf Score(A) = 42 Then\nIf B.Position.X = 1 Then\nReturn 0\nEnd If\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_tuple_construction_copy_and_projection() {
    execute(
        "Function Main() As Integer\nDim A = (10, 2.5)\nDim B = A\nIf A.Item1 = 10 Then\nIf B.Item1 = 10 Then\nIf A.Item2 = 2.5 Then\nReturn 0\nEnd If\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_tuple_readonly_borrow_uses_caller_storage() {
    execute(
        "Function First(ByRef ReadOnly Pair As (Integer, Double)) As Integer\nReturn Pair.Item1\nEnd Function\nFunction Main() As Integer\nDim Pair = (42, 2.5)\nReturn First(Pair)\nEnd Function",
        42,
    );
}

#[test]
fn native_array_index_copy_and_snapshot_foreach() {
    execute(
        "Function Main() As Integer\nDim Values(1) As Integer\nValues(0) = 1\nValues(1) = 2\nDim Item As Integer = 0\nDim Total As Integer = 0\nFor Each Item In Values\nTotal += Item\nValues(1) = 9\nNext\nIf Total = 3 Then\nIf Values(1) = 9 Then\nReturn 0\nEnd If\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_array_of_structures_and_nested_index_field_place() {
    execute(
        "Structure Point\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nFunction Main() As Integer\nDim Points(1) As Point\nPoints(0).X = 3\nPoints(1).Y = 6\nIf Points(0).X + Points(1).Y = 9 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_foreach_over_plain_structures_copies_entry_snapshot() {
    execute(
        "Structure Point\nPublic X As Integer\nEnd Structure\nFunction Main() As Integer\nDim Points(1) As Point\nPoints(0).X = 20\nPoints(1).X = 22\nDim Item As Point\nDim Total As Integer = 0\nFor Each Item In Points\nTotal += Item.X\nPoints(1).X = 1\nNext\nReturn Total\nEnd Function",
        42,
    );
}

#[test]
fn empty_structure_has_distinct_native_identity() {
    execute(
        "Structure Marker\nEnd Structure\nFunction Main() As Integer\nDim M As Marker\nReturn 0\nEnd Function",
        0,
    );
}

#[test]
fn structure_with_class_field_uses_managed_copy_and_drop() {
    execute(
        "Class Resource\nPublic Value As Integer\nEnd Class\nStructure Holder\nPublic Ref As Resource\nEnd Structure\nFunction Main() As Integer\nDim R As New Resource()\nR.Value = 42\nDim H As Holder\nH.Ref = R\nDim Copy As Holder = H\nR = Nothing\nIf Copy.Ref.Value = 42 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn readonly_structure_parameter_rejects_projected_write_before_mir() {
    let source = "Structure Point\nPublic X As Integer\nEnd Structure\nFunction Bad(ByRef ReadOnly P As Point) As Integer\nP.X = 1\nReturn 0\nEnd Function";
    let program = parse_source(source).unwrap();
    let error = valo_core::semantics::validate_snippet(&program).unwrap_err();
    assert!(
        error.message.contains("Cannot modify ByRef ReadOnly"),
        "{error:?}"
    );
}

#[test]
fn native_array_bounds_violation_traps() {
    let Some(tools) = tools() else { return };
    let program = module(
        "Function Main() As Integer\nDim Values(1) As Integer\nReturn Values(0 - 1)\nEnd Function",
    );
    let path = output("bounds-trap");
    build(
        &program,
        &tools,
        &NativeOptions {
            output: path.clone(),
            kind: EmitKind::Executable,
            optimize: false,
        },
    )
    .unwrap();
    let status = Command::new(&path).status().unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(!status.success());
}

#[test]
fn native_mutable_byref_struct_and_projected_scalar_borrow() {
    execute(
        "Structure Point\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nFunction Increment(ByRef X As Integer) As Integer\nX += 1\nReturn X\nEnd Function\nFunction Change(ByRef P As Point) As Integer\nP.Y = Increment(P.X)\nReturn P.X + P.Y\nEnd Function\nFunction Main() As Integer\nDim P As Point\nP.X = 20\nIf Change(P) = 42 Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_boolean_fields_and_fixed_array_elements() {
    execute(
        "Structure Flags\nPublic Ready As Boolean\nPublic Count As Integer\nEnd Structure\nFunction Main() As Integer\nDim F As Flags\nDim Bits(0) As Boolean\nF.Ready = True\nBits(0) = F.Ready\nIf Bits(0) Then\nReturn 0\nEnd If\nReturn 1\nEnd Function",
        0,
    );
}

#[test]
fn native_foreach_snapshot_matches_interpreter_result() {
    let source = "Function Main() As Integer\nDim Values(1) As Integer\nValues(0) = 1\nValues(1) = 2\nDim Item As Integer = 0\nDim Total As Integer = 0\nFor Each Item In Values\nTotal += Item\nValues(1) = 9\nNext\nReturn Total\nEnd Function";
    let interpreted = source.replace("Function Main() As Integer", "Function Result() As Integer")
        + "\nSub Main()\nConsole.WriteLine(Result())\nEnd Sub";
    assert_eq!(valo_core::run_source(&interpreted).unwrap(), ["3"]);
    execute(source, 3);
}

#[test]
fn native_structure_and_tuple_values_match_interpreter() {
    for source in [
        "Structure Vec2\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nFunction Main() As Integer\nDim V As Vec2\nV.X = 20\nV.Y = 22\nReturn V.X + V.Y\nEnd Function",
        "Function Main() As Integer\nDim Pair = (20, 22)\nReturn Pair.Item1 + Pair.Item2\nEnd Function",
    ] {
        let interpreted = source
            .replace("Function Main() As Integer", "Function Result() As Integer")
            + "\nSub Main()\nConsole.WriteLine(Result())\nEnd Sub";
        assert_eq!(valo_core::run_source(&interpreted).unwrap(), ["42"]);
        execute(source, 42);
    }
}

#[test]
fn llvm_ir_records_aggregate_layout_and_resolved_geps() {
    let source = "Structure Mixed\nPublic B As Byte\nPublic Number As Long\nPublic Flag As Boolean\nEnd Structure\nFunction Main() As Integer\nDim V As Mixed\nV.Number = 42\nV.Flag = True\nIf V.Flag Then\nReturn 0\nEnd If\nReturn 1\nEnd Function";
    let target = valo_core::backend::llvm::Target {
        triple: "x86_64-pc-windows-msvc".into(),
        data_layout:
            "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128".into(),
        pointer_bits: 64,
    };
    let text = render_module(&module(source), &target).unwrap();
    assert!(text.contains("type { i8, i64, i1 }"), "{text}");
    assert!(text.contains("getelementptr inbounds %valo_t0"), "{text}");
}
