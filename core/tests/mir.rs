use valo_core::mir::{debug, ir, lower_body, verify};
use valo_core::semantics::lower_function_body;
use valo_core::{TypeName, parse_source};

fn mir(source: &str) -> ir::Function {
    let program = parse_source(source).unwrap();
    let hir = lower_function_body(&program, 0).unwrap();
    lower_body(&hir).unwrap()
}

#[test]
fn literal_return_has_a_typed_temp_and_terminator() {
    let function = mir("Function F() As Integer\nReturn 7\nEnd Function");
    assert_eq!(function.temps, vec![TypeName::Int32]);
    assert_eq!(function.blocks.len(), 1);
    assert_eq!(
        debug::format_function(&function),
        "fn #0 F() -> Int32\nbb0: ; entry\n  %0: Int32 = const Integer(7)\n  return %0\n"
    );
}

#[test]
fn arithmetic_and_widening_cast_are_typed() {
    let function = mir("Function Mix(A As Integer, B As Long) As Long\nReturn A + B\nEnd Function");
    let dump = debug::format_function(&function);
    assert!(dump.contains("cast.NumericChecked"), "{dump}");
    assert!(dump.contains("Add %"), "{dump}");
    assert!(dump.contains("-> Int64"), "{dump}");
    verify::verify(&function).unwrap();
}

#[test]
fn primitive_arithmetic_operations_keep_resolved_types() {
    for (operator, opcode) in [
        ("-", "Subtract"),
        ("*", "Multiply"),
        ("\\", "IntegerDivide"),
        ("Mod", "Modulo"),
    ] {
        let source = format!(
            "Function F(A As Integer, B As Integer) As Integer\nReturn A {operator} B\nEnd Function"
        );
        let dump = debug::format_function(&mir(&source));
        assert!(dump.contains(opcode), "{operator}: {dump}");
    }
    let dump = debug::format_function(&mir(
        "Function F(A As Integer, B As Integer) As Double\nReturn A / B\nEnd Function",
    ));
    assert!(dump.contains("Divide"), "{dump}");
    assert!(dump.contains("Double"), "{dump}");
}

#[test]
fn resolved_function_call_uses_identity_and_signature() {
    let source = "Function Add(A As Integer, B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use() As Integer\nReturn Add(2, 3)\nEnd Function";
    let program = parse_source(source).unwrap();
    let hir = lower_function_body(&program, 1).unwrap();
    let function = lower_body(&hir).unwrap();
    let dump = debug::format_function(&function);
    assert!(dump.contains("call fn#0(%0, %1)"), "{dump}");
    let call = &function.blocks[0].instructions[2].kind;
    let ir::InstructionKind::Call {
        parameter_types,
        parameter_modes,
        ..
    } = call
    else {
        panic!()
    };
    assert_eq!(parameter_types, &[TypeName::Int32, TypeName::Int32]);
    assert_eq!(parameter_modes.len(), 2);
}

#[test]
fn module_collects_verified_functions_in_source_order() {
    let source = "Function A() As Integer\nReturn 1\nEnd Function\nFunction B() As Integer\nReturn 2\nEnd Function";
    let program = parse_source(source).unwrap();
    let bodies = (0..2)
        .map(|index| lower_function_body(&program, index).unwrap())
        .collect::<Vec<_>>();
    let module = valo_core::mir::lower_module(&bodies).unwrap();
    assert_eq!(
        module
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        ["A", "B"]
    );
}

#[test]
fn if_else_creates_branch_and_return_blocks() {
    let function = mir(
        "Function Max(A As Integer, B As Integer) As Integer\nIf A > B Then\nReturn A\nElse\nReturn B\nEnd If\nEnd Function",
    );
    let dump = debug::format_function(&function);
    assert!(dump.contains("cmp.Greater"), "{dump}");
    assert!(dump.contains("branch %"), "{dump}");
    assert!(dump.contains("bb1: ; if.then"), "{dump}");
    assert!(dump.contains("bb2: ; if.else"), "{dump}");
    assert_eq!(dump.matches("return %").count(), 2);
}

#[test]
fn while_do_and_for_have_explicit_backedges() {
    let function = mir(
        "Function Count(N As Integer) As Integer\nDim I As Integer = 0\nWhile I < N\nI += 1\nWend\nDo While I < N\nI += 1\nLoop\nFor I = 0 To N Step 1\nIf I = 1 Then\nContinue For\nEnd If\nIf I = 2 Then\nExit For\nEnd If\nNext\nReturn I\nEnd Function",
    );
    let dump = debug::format_function(&function);
    for label in [
        "while.test",
        "while.body",
        "do.test",
        "do.body",
        "for.test",
        "for.step",
        "for.exit",
    ] {
        assert!(dump.contains(label), "missing {label}: {dump}");
    }
    assert!(dump.matches("goto bb").count() >= 6, "{dump}");
    assert!(dump.contains("trap \"For Step cannot be zero\""), "{dump}");
}

#[test]
fn supported_fixed_array_foreach_lowers_to_indexed_cfg() {
    let function = mir(
        "Function Sum() As Integer\nDim Values(2) As Integer\nDim Item As Integer = 0\nDim Total As Integer = 0\nFor Each Item In Values\nTotal += Item\nNext\nReturn Total\nEnd Function",
    );
    let dump = debug::format_function(&function);
    for text in [
        "array.init 0..=2",
        "array.len $0",
        "foreach.test",
        "foreach.step",
        "load $0[%",
    ] {
        assert!(dump.contains(text), "missing {text}: {dump}");
    }
}

#[test]
fn resolved_field_place_and_byref_mode_survive_lowering() {
    let source = "Structure Point\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nFunction Add(ByRef A As Integer, ByRef ReadOnly B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use(ByRef P As Point) As Integer\nP.X = 7\nReturn Add(P.X, P.Y)\nEnd Function";
    let program = parse_source(source).unwrap();
    let hir = lower_function_body(&program, 1).unwrap();
    let function = lower_body(&hir).unwrap();
    let dump = debug::format_function(&function);
    assert!(dump.contains("store $0.field#0"), "{dump}");
    assert!(dump.contains("BorrowMutable $0.field#0"), "{dump}");
    assert!(dump.contains("BorrowImmutable $0.field#1"), "{dump}");
}

#[test]
fn indexed_store_retains_typed_place() {
    let function = mir(
        "Function Use() As Integer\nDim Values(2) As Integer\nValues(1) = 7\nReturn Values(1)\nEnd Function",
    );
    let dump = debug::format_function(&function);
    assert!(dump.contains("store $0[%"), "{dump}");
    assert!(dump.contains("load $0[%"), "{dump}");
}

#[test]
fn return_value_precedes_finally_then_dispose() {
    let function = mir(
        "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nUsing R\nTry\nReturn 7\nFinally\nDim X As Integer = 1\nEnd Try\nEnd Using\nEnd Function",
    );
    let dump = debug::format_function(&function);
    let value = dump.find("const Integer(7)").unwrap();
    let finally = dump.find("cleanup.finally").unwrap();
    let dispose = dump.find("cleanup.dispose").unwrap();
    let return_at = dump.find("return %").unwrap();
    assert!(
        value < finally && finally < dispose && dispose < return_at,
        "{dump}"
    );
    assert_eq!(dump.matches("call dispose#").count(), 1);
}

#[test]
fn nested_cleanup_order_is_finally_inner_then_outer_dispose() {
    let function = mir(
        "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(A As Resource, B As Resource) As Integer\nUsing A\nUsing B\nTry\nReturn 7\nFinally\nDim X As Integer = 1\nEnd Try\nEnd Using\nEnd Using\nEnd Function",
    );
    let dump = debug::format_function(&function);
    assert_eq!(dump.matches("call dispose#").count(), 2, "{dump}");
    let finally = dump.find("cleanup.finally").unwrap();
    let first = dump.find("call dispose#").unwrap();
    let second = dump[first + 1..].find("call dispose#").unwrap() + first + 1;
    assert!(finally < first && first < second, "{dump}");
}

#[test]
fn normal_try_finally_flows_through_cleanup_before_following_return() {
    let function = mir(
        "Function F() As Integer\nDim X As Integer = 1\nTry\nX = 2\nFinally\nX = 3\nEnd Try\nReturn X\nEnd Function",
    );
    let dump = debug::format_function(&function);
    let handler = dump.find("cleanup.finally").unwrap();
    let returned = dump.rfind("return %").unwrap();
    assert!(handler < returned, "{dump}");
    assert_eq!(dump.matches("cleanup.finally").count(), 1);
}

#[test]
fn continue_and_exit_run_using_cleanup_before_loop_destination() {
    let function = mir(
        "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction F(R As Resource) As Integer\nDim I As Integer = 0\nFor I = 0 To 2\nUsing R\nIf I = 0 Then\nContinue For\nEnd If\nExit For\nEnd Using\nNext\nReturn I\nEnd Function",
    );
    let dump = debug::format_function(&function);
    assert_eq!(dump.matches("call dispose#").count(), 2, "{dump}");
    assert!(dump.contains("for.step"), "{dump}");
    assert!(dump.contains("for.exit"), "{dump}");
}

#[test]
fn continue_from_using_inside_try_disposes_then_runs_finally() {
    let function = mir(
        "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction F(R As Resource) As Integer\nDim I As Integer = 0\nFor I = 0 To 1\nTry\nUsing R\nContinue For\nEnd Using\nFinally\nI += 1\nEnd Try\nNext\nReturn I\nEnd Function",
    );
    let dump = debug::format_function(&function);
    let dispose = dump.find("cleanup.dispose").unwrap();
    let finally = dump.find("cleanup.finally").unwrap();
    assert!(dispose < finally, "{dump}");
}

#[test]
fn catch_requires_exception_dispatch_instead_of_fake_cfg() {
    let program = parse_source(
        "Function F() As Integer\nTry\nReturn 1\nCatch\nReturn 2\nEnd Try\nEnd Function",
    )
    .unwrap();
    let hir = lower_function_body(&program, 0).unwrap();
    assert!(matches!(
        lower_body(&hir),
        Err(valo_core::mir::LowerError::Unsupported {
            feature: "native Catch dispatch and exception edges",
            ..
        })
    ));
}

#[test]
fn verifier_rejects_bad_targets_returns_and_missing_terminators() {
    let original = mir("Function F() As Integer\nReturn 1\nEnd Function");
    let mut bad = original.clone();
    bad.blocks[0].terminator = Some(ir::Terminator {
        kind: ir::TerminatorKind::Goto(ir::BlockId(99)),
        span: bad.span,
    });
    assert!(verify::verify(&bad).unwrap_err().contains("branch target"));
    let mut bad = original.clone();
    bad.blocks[0].terminator = None;
    assert!(verify::verify(&bad).unwrap_err().contains("no terminator"));
    let mut bad = original;
    bad.return_type = TypeName::Boolean;
    assert!(verify::verify(&bad).unwrap_err().contains("return type"));
}

#[test]
fn verifier_rejects_non_boolean_branch_and_mistyped_store() {
    let original = mir(
        "Function F() As Integer\nDim X As Integer = 1\nIf X > 0 Then\nReturn X\nEnd If\nReturn 0\nEnd Function",
    );
    let mut bad = original.clone();
    let integer_temp = bad.blocks[0].instructions[0].result.unwrap();
    let Some(ir::Terminator {
        kind: ir::TerminatorKind::Branch { condition, .. },
        ..
    }) = &mut bad.blocks[0].terminator
    else {
        panic!()
    };
    *condition = integer_temp;
    assert!(verify::verify(&bad).unwrap_err().contains("not Boolean"));

    let mut bad = original;
    let ir::InstructionKind::Store { value, .. } = &mut bad.blocks[0].instructions[1].kind else {
        panic!()
    };
    *value = ir::TempId(99);
    assert!(verify::verify(&bad).unwrap_err().contains("temp ID"));
}
