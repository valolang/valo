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
    let status = Command::new(&path).status().unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(status.code(), Some(expected));
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
fn class_dynamic_and_native_drop_remain_explicitly_unsupported() {
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
    assert!(
        render_module(&program, &tools.target)
            .unwrap_err()
            .message
            .contains("native primitive ABI")
    );

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
    assert!(error.message.contains("native Drop contract"), "{error}");
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
fn structure_with_class_field_is_not_assumed_copy_or_dropless() {
    let Some(tools) = tools() else { return };
    let program = module(
        "Class Resource\nEnd Class\nStructure Holder\nPublic Value As Resource\nEnd Structure\nFunction Main() As Integer\nDim H As Holder\nReturn 0\nEnd Function",
    );
    let error = render_module(&program, &tools.target).unwrap_err();
    assert_eq!(error.stage, "native eligibility");
    assert!(error.message.contains("ownership requirements"), "{error}");
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
