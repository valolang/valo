use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

use valo_core::backend::llvm::{EmitKind, LlvmTools, NativeOptions, build, render_module};
use valo_core::mir::{ir, lower_module};
use valo_core::parse_source;
use valo_core::semantics::lower_function_body;

fn module(source: &str) -> ir::Module {
    let program = parse_source(source).unwrap();
    let bodies = (0..program.functions.len())
        .map(|index| lower_function_body(&program, index).unwrap())
        .collect::<Vec<_>>();
    lower_module(&bodies).unwrap()
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
    assert!(error.message.contains("native primitive ABI"));

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
    assert!(ir.contains("define i32 @valo_f0"));
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
