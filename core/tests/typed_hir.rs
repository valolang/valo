use valo_core::semantics::arithmetic::{self, ArithmeticOp};
use valo_core::semantics::debug_hir;
use valo_core::semantics::ownership;
use valo_core::semantics::type_properties::KnownProperty;
use valo_core::semantics::verify_hir;
use valo_core::semantics::{lower_function_body, typed_hir::*};
use valo_core::{TypeName, parse_source};

fn body(source: &str, index: usize) -> TypedBody {
    lower_function_body(&parse_source(source).unwrap(), index).unwrap()
}
#[test]
fn verifier_rejects_field_without_declared_structure_owner() {
    let mut hir = body(
        "Structure Point\nPublic X As Integer\nEnd Structure\nFunction Main() As Integer\nDim P As Point\nReturn 0\nEnd Function",
        0,
    );
    hir.fields[0].owner = TypeName::User("Missing".into());
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("field owner")
    );
}
fn returned(body: &TypedBody) -> &Expression {
    match body.statements.last().unwrap() {
        Statement::Return { value, .. } => value,
        _ => panic!("expected return"),
    }
}

#[test]
fn arithmetic_has_canonical_types_and_resolved_places() {
    let source = "Function Add(A As Integer, B As Int32) As Integer\nDim X As Integer = A\nX += B\nReturn X\nEnd Function";
    let hir = body(source, 0);
    assert_eq!(hir, body(source, 0));
    assert_eq!(hir.locals.len(), 3);
    assert_eq!(hir.locals[0].parameter_index, Some(0));
    assert_eq!(hir.locals[0].storage, LocalStorage::Value);
    assert_eq!(hir.locals[2].ty, TypeName::Int32);
    let Statement::Store { target, value, .. } = &hir.statements[1] else {
        panic!()
    };
    assert_eq!(target.category, ValueCategory::Place);
    assert_eq!(target.kind, ExpressionKind::Place(Place::local(LocalId(2))));
    let ExpressionKind::Arithmetic {
        operation,
        signature,
        left,
        right,
    } = &value.kind
    else {
        panic!()
    };
    assert_eq!(*operation, ArithmeticOp::Add);
    assert_eq!(signature.result_type, TypeName::Int32);
    assert_eq!(left.ty, TypeName::Int32);
    assert_eq!(right.ty, TypeName::Int32);
    assert!(matches!(left.kind, ExpressionKind::Load(_)));
}

#[test]
fn widening_and_return_conversions_are_explicit() {
    let hir = body(
        "Function Mix(A As Integer, B As Long) As Double\nReturn A + B\nEnd Function",
        0,
    );
    let ExpressionKind::Convert { value, conversion } = &returned(&hir).kind else {
        panic!()
    };
    assert_eq!(*conversion, Conversion::NumericChecked);
    assert_eq!(returned(&hir).ty, TypeName::Double);
    let ExpressionKind::Arithmetic {
        signature,
        left,
        right,
        ..
    } = &value.kind
    else {
        panic!()
    };
    assert_eq!(signature.result_type, TypeName::Int64);
    assert!(matches!(left.kind, ExpressionKind::Convert { .. }));
    assert_eq!(left.ty, right.ty);
}

#[test]
fn overload_identity_is_stored_in_the_call() {
    let hir = body(
        r#"
Function Pick(Value As Integer) As Integer
    Return Value
End Function
Function Pick(Value As Long) As Long
    Return Value
End Function
Function Use(A As Integer) As Long
    Return Pick(A + 1)
End Function
"#,
        2,
    );
    let ExpressionKind::Convert { value, .. } = &returned(&hir).kind else {
        panic!()
    };
    let ExpressionKind::Call {
        function,
        signature,
        arguments,
    } = &value.kind
    else {
        panic!()
    };
    assert_eq!(*function, BodyFunctionId(0));
    assert_eq!(signature.parameter_types, vec![TypeName::Int32]);
    assert_eq!(arguments[0].value.ty, TypeName::Int32);
    assert_eq!(arguments[0].mode, ArgumentMode::ByVal);
}

#[test]
fn byref_parameters_and_call_arguments_are_explicit_borrows() {
    let source = r#"
Function Update(ByRef Value As Integer) As Integer
    Value += 1
    Return Value
End Function
Function Use(ByRef Value As Integer) As Integer
    Return Update(Value)
End Function
"#;
    let hir = body(source, 1);
    assert_eq!(hir.locals[0].storage, LocalStorage::BorrowedMutable);
    let ExpressionKind::Call { arguments, .. } = &returned(&hir).kind else {
        panic!()
    };
    assert_eq!(arguments[0].mode, ArgumentMode::BorrowMutable);
    assert_eq!(arguments[0].value.category, ValueCategory::MutableReference);
    assert!(matches!(
        arguments[0].value.kind,
        ExpressionKind::BorrowMutable(_)
    ));
    assert_eq!(body(source, 0).locals[0].parameter_index, Some(0));
}

#[test]
fn readonly_byref_calls_record_immutable_borrows_and_check_conflicts() {
    let source = "Function Pair(ByRef ReadOnly A As Integer, ByRef ReadOnly B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use() As Integer\nDim X As Integer = 2\nReturn Pair(X, X)\nEnd Function";
    let hir = body(source, 1);
    let ExpressionKind::Call { arguments, .. } = &returned(&hir).kind else {
        panic!()
    };
    assert_eq!(arguments[0].mode, ArgumentMode::BorrowImmutable);
    assert_eq!(arguments[1].mode, ArgumentMode::BorrowImmutable);
    assert!(matches!(
        arguments[0].value.kind,
        ExpressionKind::BorrowImmutable(_)
    ));
    assert_eq!(
        body(source, 0).locals[0].storage,
        LocalStorage::BorrowedImmutable
    );

    let mixed = source.replace("ByRef ReadOnly A As Integer", "ByRef A As Integer");
    let error = lower_function_body(&parse_source(&mixed).unwrap(), 1).unwrap_err();
    assert!(error.message.contains("conflicting ByRef parameters"));
}

#[test]
fn integer_division_records_truncation() {
    let hir = body(
        r#"Function Divide(A As Double, B As Double) As Long
Return A \ B
End Function"#,
        0,
    );
    let ExpressionKind::Arithmetic { left, right, .. } = &returned(&hir).kind else {
        panic!()
    };
    for operand in [left, right] {
        assert!(matches!(
            operand.kind,
            ExpressionKind::Convert {
                conversion: Conversion::TruncateToInteger,
                ..
            }
        ));
        assert_eq!(operand.ty, TypeName::Int64);
    }
}

#[test]
fn while_body_has_a_lexical_scope_and_return_unwinds_it() {
    let program = parse_source(
        "Function Choose(A As Boolean) As Integer\nWhile A\nReturn 1\nWend\nReturn 2\nEnd Function",
    )
    .unwrap();
    valo_core::semantics::validate_snippet(&program).unwrap();
    let hir = lower_function_body(&program, 0).unwrap();
    let Statement::While {
        body_scope, body, ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_eq!(hir.scopes[body_scope.0].parent, Some(hir.root_scope));
    let Statement::Return { exited_scopes, .. } = &body[0] else {
        panic!()
    };
    assert_eq!(exited_scopes, &vec![*body_scope, hir.root_scope]);
}

#[test]
fn conditional_paths_have_typed_comparisons_and_complete_returns() {
    let hir = body(
        "Function Choose(A As Integer, B As Long) As Integer\nIf A < B Then\nReturn 1\nElseIf A = 0 Then\nReturn 2\nElse\nReturn 3\nEnd If\nEnd Function",
        0,
    );
    assert_eq!(hir.statements.len(), 1);
    let Statement::If {
        condition,
        then_body,
        else_body,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_eq!(condition.ty, TypeName::Boolean);
    let ExpressionKind::Compare {
        operation,
        left,
        right,
    } = &condition.kind
    else {
        panic!()
    };
    assert_eq!(*operation, ComparisonOp::Less);
    assert_eq!(left.ty, TypeName::Int64);
    assert_eq!(right.ty, TypeName::Int64);
    assert!(matches!(left.kind, ExpressionKind::Convert { .. }));
    assert!(matches!(then_body[0], Statement::Return { .. }));
    let Statement::If {
        condition,
        else_body,
        ..
    } = &else_body[0]
    else {
        panic!()
    };
    assert!(matches!(
        condition.kind,
        ExpressionKind::Compare {
            operation: ComparisonOp::Equal,
            ..
        }
    ));
    assert!(matches!(else_body[0], Statement::Return { .. }));
}

#[test]
fn conditional_fallthrough_and_unreachable_code_are_rejected() {
    let missing_return = parse_source(
        "Function Choose(A As Boolean) As Integer\nIf A Then\nReturn 1\nEnd If\nEnd Function",
    )
    .unwrap();
    assert!(
        lower_function_body(&missing_return, 0)
            .unwrap_err()
            .message
            .contains("path that does not Return")
    );

    let unreachable = parse_source("Function Choose(A As Boolean) As Integer\nIf A Then\nReturn 1\nElse\nReturn 2\nEnd If\nReturn 3\nEnd Function").unwrap();
    assert!(
        lower_function_body(&unreachable, 0)
            .unwrap_err()
            .message
            .contains("unreachable")
    );
}

#[test]
fn conditional_stores_join_at_the_following_return() {
    let hir = body(
        "Function Choose(A As Boolean) As Integer\nDim X As Integer = 0\nIf A Then\nX = 1\nElse\nX = 2\nEnd If\nReturn X\nEnd Function",
        0,
    );
    let Statement::If {
        then_body,
        else_body,
        ..
    } = &hir.statements[1]
    else {
        panic!()
    };
    for branch in [then_body, else_body] {
        let Statement::Store { target, .. } = &branch[0] else {
            panic!()
        };
        assert_eq!(target.kind, ExpressionKind::Place(Place::local(LocalId(1))));
    }
    assert!(matches!(hir.statements[2], Statement::Return { .. }));
}

#[test]
fn branch_local_declarations_have_separate_lifetime_scopes() {
    let source = "Function Choose(A As Boolean) As Integer\nIf A Then\nDim X As Integer = 1\nReturn X\nElse\nReturn 0\nEnd If\nEnd Function";
    let program = parse_source(source).unwrap();
    let hir = lower_function_body(&program, 0).unwrap();
    let Statement::If {
        then_scope,
        else_scope,
        then_body,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_ne!(then_scope, else_scope);
    assert_eq!(hir.locals[1].scope, *then_scope);
    assert_eq!(hir.scopes[then_scope.0].locals, vec![LocalId(1)]);
    let Statement::Return { exited_scopes, .. } = &then_body[1] else {
        panic!()
    };
    assert_eq!(exited_scopes, &vec![*then_scope, hir.root_scope]);
}

#[test]
fn branch_local_binding_does_not_escape_its_scope() {
    let hir = body(
        "Function Choose(A As Boolean) As Integer\nIf A Then\nDim X As Integer = 2\nReturn X\nElse\nDim Y As Integer = 3\nReturn Y\nEnd If\nEnd Function",
        0,
    );
    let Statement::If {
        then_scope,
        else_scope,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_eq!(hir.scopes[then_scope.0].locals, vec![LocalId(1)]);
    assert_eq!(hir.scopes[else_scope.0].locals, vec![LocalId(2)]);
    assert_eq!(hir.locals[1].initial_state, InitialState::Uninitialized);
    assert_eq!(hir.locals[2].initial_state, InitialState::Uninitialized);

    let escaping = parse_source("Function Invalid(A As Boolean) As Integer\nIf A Then\nDim X As Integer = 1\nEnd If\nReturn X\nEnd Function").unwrap();
    assert!(
        lower_function_body(&escaping, 0)
            .unwrap_err()
            .message
            .contains("non-local place")
    );
}

#[test]
fn counted_loop_exit_and_continue_record_cleanup_scopes() {
    let hir = body(
        "Function Count(N As Integer) As Integer\nDim I As Integer = 0\nFor I = 0 To N Step 1\nDim Temp As Integer = I\nIf I = 1 Then\nContinue For\nEnd If\nIf I = 3 Then\nExit For\nEnd If\nNext\nReturn I\nEnd Function",
        0,
    );
    let Statement::For {
        id,
        variable,
        start,
        end,
        step,
        body_scope,
        body,
        ..
    } = &hir.statements[1]
    else {
        panic!()
    };
    assert_eq!(*variable, LocalId(1));
    assert_eq!(start.ty, TypeName::Int32);
    assert_eq!(end.ty, TypeName::Int32);
    assert_eq!(step.ty, TypeName::Int32);
    for (branch, is_continue) in [(1, true), (2, false)] {
        let Statement::If {
            then_scope,
            then_body,
            ..
        } = &body[branch]
        else {
            panic!()
        };
        let (jump_id, exited) = match &then_body[0] {
            Statement::ContinueLoop {
                loop_id,
                exited_scopes,
                ..
            } if is_continue => (loop_id, exited_scopes),
            Statement::ExitLoop {
                loop_id,
                exited_scopes,
                ..
            } if !is_continue => (loop_id, exited_scopes),
            _ => panic!(),
        };
        assert_eq!(jump_id, id);
        assert_eq!(exited, &vec![*then_scope, *body_scope]);
    }
    let report = ownership::analyze_body(&hir).unwrap();
    for kind in [
        ownership::ExitKind::ContinueLoop,
        ownership::ExitKind::ExitLoop,
    ] {
        assert!(report.exits.iter().any(|exit| exit.kind == kind
            && exit.locals == vec![(LocalId(2), ownership::CleanupAction::NoDrop)]));
    }
}

#[test]
fn do_loop_preserves_pre_and_post_test_positions() {
    let hir = body(
        "Function Count() As Integer\nDim I As Integer = 0\nDo While I < 2\nI += 1\nLoop\nDo\nI += 1\nLoop Until I = 3\nReturn I\nEnd Function",
        0,
    );
    assert!(matches!(
        hir.statements[1],
        Statement::Do {
            condition: DoCondition::PreWhile(_),
            ..
        }
    ));
    assert!(matches!(
        hir.statements[2],
        Statement::Do {
            condition: DoCondition::PostUntil(_),
            ..
        }
    ));
}

#[test]
fn foreach_records_array_element_type_and_body_scope() {
    let hir = body(
        "Function Sum() As Integer\nDim Values(2) As Integer\nDim Item As Integer = 0\nDim Total As Integer = 0\nFor Each Item In Values\nTotal += Item\nNext\nReturn Total\nEnd Function",
        0,
    );
    let Statement::ForEach {
        variable,
        iterable,
        element_type,
        body_scope,
        ..
    } = &hir.statements[3]
    else {
        panic!()
    };
    assert_eq!(*variable, LocalId(1));
    assert_eq!(*element_type, TypeName::Int32);
    assert_eq!(iterable.ty, TypeName::Array(Box::new(TypeName::Int32)));
    assert_eq!(hir.scopes[body_scope.0].parent, Some(hir.root_scope));
}

#[test]
fn simultaneous_mutable_byref_aliases_are_rejected_in_typed_hir() {
    let source = "Function Sum(ByRef A As Integer, ByRef B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use() As Integer\nDim X As Integer = 1\nReturn Sum(X, X)\nEnd Function";
    let program = parse_source(source).unwrap();
    let error = lower_function_body(&program, 1).unwrap_err();
    assert!(error.message.contains("conflicting ByRef parameters"));
    assert!(error.span.is_some());

    let distinct = source.replace("Return Sum(X, X)", "Dim Y As Integer = 2\nReturn Sum(X, Y)");
    assert!(body(&distinct, 1).statements.len() >= 2);
}

#[test]
fn selected_types_depend_on_operands_not_runtime_magnitudes() {
    for (left, right, expected) in [
        (TypeName::Int32, TypeName::Int32, TypeName::Int32),
        (TypeName::Int32, TypeName::Int64, TypeName::Int64),
        (TypeName::Single, TypeName::Int32, TypeName::Single),
        (TypeName::UInt32, TypeName::Int32, TypeName::Int64),
        (TypeName::UInt64, TypeName::UInt32, TypeName::UInt64),
    ] {
        assert_eq!(
            arithmetic::signature(ArithmeticOp::Add, &left, &right)
                .unwrap()
                .result_type,
            expected
        );
    }
    assert!(
        arithmetic::signature(ArithmeticOp::Add, &TypeName::UInt64, &TypeName::Int32).is_none()
    );
}

#[test]
fn hir_debug_text_is_stable_and_shows_scopes_and_borrow_modes() {
    let source = "Function Inspect(ByRef ReadOnly X As Integer) As Integer\nIf X > 0 Then\nReturn X\nElse\nReturn 0\nEnd If\nEnd Function";
    let hir = body(source, 0);
    let printed = debug_hir::format_body(&hir);
    assert_eq!(printed, debug_hir::format_body(&body(source, 0)));
    assert!(printed.contains("local #0 X: Int32 BorrowedImmutable Initialized"));
    assert!(printed.contains("cmp.Greater"));
    assert!(printed.contains("return load(local #0:Int32):Int32 exits [#1, #0]"));
}

#[test]
fn hir_verifier_rejects_call_passing_mode_mismatch() {
    let source = "Function Read(ByRef ReadOnly X As Integer) As Integer\nReturn X\nEnd Function\nFunction Use() As Integer\nDim X As Integer = 1\nReturn Read(X)\nEnd Function";
    let mut hir = body(source, 1);
    let Statement::Return { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Call { signature, .. } = &mut value.kind else {
        panic!()
    };
    signature.parameter_modes[0] = ArgumentMode::BorrowMutable;
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("passing mode")
    );
}

#[test]
fn place_overlap_is_conservative_for_fields_and_indices() {
    let root = Place::local(LocalId(0));
    let field = |id| Place {
        root: LocalId(0),
        projections: vec![Projection::Field(FieldId(id))],
    };
    let index = |key| Place {
        root: LocalId(0),
        projections: vec![Projection::Index(IndexProjection {
            index: Box::new(Expression {
                kind: match key {
                    IndexKey::Constant(value) => ExpressionKind::Constant(Constant::Integer(value)),
                    IndexKey::Unknown => ExpressionKind::Place(Place::local(LocalId(1))),
                },
                ty: TypeName::Int32,
                category: ValueCategory::Value,
                span: valo_core::runtime::Span::empty(valo_core::runtime::FileId(0)),
            }),
            element_type: TypeName::Int32,
        })],
    };
    assert_eq!(root.overlap(&field(0)), PlaceOverlap::Overlap);
    assert_eq!(field(0).overlap(&field(1)), PlaceOverlap::Disjoint);
    assert_eq!(field(0).overlap(&field(0)), PlaceOverlap::Overlap);
    assert_eq!(
        index(IndexKey::Constant(0)).overlap(&index(IndexKey::Constant(1))),
        PlaceOverlap::Disjoint
    );
    assert_eq!(
        index(IndexKey::Unknown).overlap(&index(IndexKey::Constant(1))),
        PlaceOverlap::Unknown
    );
    assert_eq!(
        root.overlap(&Place::local(LocalId(1))),
        PlaceOverlap::Disjoint
    );
}

#[test]
fn exit_candidates_follow_reverse_scope_and_declaration_order() {
    let hir = body(
        "Function Use() As Integer\nDim A As Integer = 1\nDim B As Integer = 2\nIf True Then\nDim C As Integer = 3\nDim D As Integer = 4\nReturn A + B + C + D\nElse\nReturn 0\nEnd If\nEnd Function",
        0,
    );
    let Statement::If { then_body, .. } = &hir.statements[2] else {
        panic!()
    };
    let Statement::Return { exited_scopes, .. } = &then_body[2] else {
        panic!()
    };
    assert_eq!(
        hir.owned_exit_locals(exited_scopes),
        vec![LocalId(3), LocalId(2), LocalId(1), LocalId(0)]
    );
    assert_eq!(
        hir.exit_drop_requirements(exited_scopes),
        vec![
            (LocalId(3), KnownProperty::No),
            (LocalId(2), KnownProperty::No),
            (LocalId(1), KnownProperty::No),
            (LocalId(0), KnownProperty::No),
        ]
    );
}

#[test]
fn hir_verifier_rejects_unresolved_projected_places() {
    let mut hir = body(
        "Function Use() As Integer\nDim X As Integer = 1\nReturn X\nEnd Function",
        0,
    );
    let Statement::Return { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Load(place) = &mut value.kind else {
        panic!()
    };
    place.kind = ExpressionKind::Place(Place {
        root: LocalId(0),
        projections: vec![Projection::Field(FieldId(0))],
    });
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("Invalid field identity")
    );
}

#[test]
fn array_element_reads_and_writes_lower_to_typed_index_places() {
    let hir = body(
        "Function Use() As Integer\nDim Items(2) As Integer\nItems(1) = 7\nReturn Items(1)\nEnd Function",
        0,
    );
    let Statement::Store { target, .. } = &hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Place(place) = &target.kind else {
        panic!()
    };
    assert_eq!(place.root, LocalId(0));
    assert_eq!(target.ty, TypeName::Int32);
    assert!(
        matches!(&place.projections[..], [Projection::Index(index)] if index.key() == IndexKey::Constant(1) && index.element_type == TypeName::Int32)
    );
    let Statement::Return { value, .. } = &hir.statements[2] else {
        panic!()
    };
    let ExpressionKind::Load(loaded) = &value.kind else {
        panic!()
    };
    let ExpressionKind::Place(returned) = &loaded.kind else {
        panic!()
    };
    assert_eq!(returned.overlap(place), PlaceOverlap::Overlap);
}

#[test]
fn indexed_byref_borrows_use_constant_index_overlap() {
    let source = "Function Add(ByRef A As Integer, ByRef B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use() As Integer\nDim Items(2) As Integer\nReturn Add(Items(0), Items(1))\nEnd Function";
    let hir = body(source, 1);
    let Statement::Return { value, .. } = &hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Call { arguments, .. } = &value.kind else {
        panic!()
    };
    assert_eq!(arguments[0].mode, ArgumentMode::BorrowMutable);
    assert_eq!(arguments[1].mode, ArgumentMode::BorrowMutable);

    let same = source.replace("Items(1)", "Items(0)");
    let error = lower_function_body(&parse_source(&same).unwrap(), 1).unwrap_err();
    assert!(error.message.contains("conflicting ByRef"));

    let dynamic = source
        .replace(
            "Dim Items(2) As Integer",
            "Dim Items(2) As Integer\nDim I As Integer = 0\nDim J As Integer = 1",
        )
        .replace("Add(Items(0), Items(1))", "Add(Items(I), Items(J))");
    let error = lower_function_body(&parse_source(&dynamic).unwrap(), 1).unwrap_err();
    assert!(error.message.contains("conflicting ByRef"));
}

#[test]
fn readonly_borrows_of_same_index_can_coexist() {
    let hir = body(
        "Function Read(ByRef ReadOnly A As Integer, ByRef ReadOnly B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use() As Integer\nDim Items(1) As Integer\nReturn Read(Items(0), Items(0))\nEnd Function",
        1,
    );
    assert!(matches!(hir.statements[1], Statement::Return { .. }));
}

#[test]
fn hir_verifier_rejects_mistyped_index_projection() {
    let mut hir = body(
        "Function Use() As Integer\nDim Items(2) As Integer\nReturn Items(0)\nEnd Function",
        0,
    );
    let Statement::Return { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Load(loaded) = &mut value.kind else {
        panic!()
    };
    let ExpressionKind::Place(place) = &mut loaded.kind else {
        panic!()
    };
    let Projection::Index(index) = &mut place.projections[0] else {
        panic!()
    };
    index.element_type = TypeName::Double;
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("Index projection")
    );
}

#[test]
fn structure_field_reads_and_writes_have_resolved_field_ids() {
    let hir = body(
        "Structure Point\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nFunction Use(ByRef P As Point) As Integer\nP.X = 7\nReturn P.Y\nEnd Function",
        0,
    );
    assert_eq!(hir.fields.len(), 2);
    let Statement::Store { target, .. } = &hir.statements[0] else {
        panic!()
    };
    let ExpressionKind::Place(written) = &target.kind else {
        panic!()
    };
    let Statement::Return { value, .. } = &hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Load(loaded) = &value.kind else {
        panic!()
    };
    let ExpressionKind::Place(read) = &loaded.kind else {
        panic!()
    };
    assert_eq!(written.overlap(read), PlaceOverlap::Disjoint);
    assert_eq!(hir.fields[0].owner, TypeName::User("Point".into()));
    assert_eq!(hir.fields[0].ty, TypeName::Int32);
}

#[test]
fn field_borrow_conflicts_distinguish_whole_and_disjoint_fields() {
    let source = "Structure Point\nPublic X As Integer\nPublic Y As Integer\nEnd Structure\nFunction Add(ByRef A As Integer, ByRef B As Integer) As Integer\nReturn A + B\nEnd Function\nFunction Use(ByRef P As Point) As Integer\nReturn Add(P.X, P.Y)\nEnd Function";
    let hir = body(source, 1);
    assert_eq!(hir.statements.len(), 1);
    let same = source.replace("Add(P.X, P.Y)", "Add(P.X, P.X)");
    assert!(
        lower_function_body(&parse_source(&same).unwrap(), 1)
            .unwrap_err()
            .message
            .contains("conflicting ByRef")
    );
}

#[test]
fn nested_structure_field_place_keeps_each_resolved_projection() {
    let hir = body(
        "Structure Inner\nPublic X As Integer\nEnd Structure\nStructure Outer\nPublic Position As Inner\nEnd Structure\nFunction Read(ByRef P As Outer) As Integer\nReturn P.Position.X\nEnd Function",
        0,
    );
    let Statement::Return { value, .. } = &hir.statements[0] else {
        panic!()
    };
    let ExpressionKind::Load(loaded) = &value.kind else {
        panic!()
    };
    let ExpressionKind::Place(place) = &loaded.kind else {
        panic!()
    };
    assert_eq!(
        place.projections,
        vec![Projection::Field(FieldId(1)), Projection::Field(FieldId(0))]
    );
    assert_eq!(loaded.ty, TypeName::Int32);
}

#[test]
fn whole_structure_and_field_mutable_borrows_conflict() {
    let source = "Structure Point\nPublic X As Integer\nEnd Structure\nFunction Touch(ByRef P As Point, ByRef X As Integer) As Integer\nReturn X\nEnd Function\nFunction Use(ByRef P As Point) As Integer\nReturn Touch(P, P.X)\nEnd Function";
    let error = lower_function_body(&parse_source(source).unwrap(), 1).unwrap_err();
    assert!(error.message.contains("conflicting ByRef"));
}

#[test]
fn tuple_field_access_has_a_typed_projection() {
    let hir = body(
        "Function Read(ByRef P As (X As Integer, Y As Integer)) As Integer\nReturn P.Y\nEnd Function",
        0,
    );
    let Statement::Return { value, .. } = &hir.statements[0] else {
        panic!()
    };
    let ExpressionKind::Load(loaded) = &value.kind else {
        panic!()
    };
    let ExpressionKind::Place(place) = &loaded.kind else {
        panic!()
    };
    assert_eq!(place.projections, vec![Projection::TupleField(1)]);
    assert_eq!(loaded.ty, TypeName::Int32);
}

#[test]
fn ownership_dataflow_rejects_use_before_initialize_and_after_move() {
    let source = "Function Use() As Integer\nDim X As Integer = 1\nReturn X + X\nEnd Function";
    let mut hir = body(source, 0);
    let Statement::Initialize { value, .. } = &mut hir.statements[0] else {
        panic!()
    };
    value.kind = ExpressionKind::Load(Box::new(Expression {
        kind: ExpressionKind::Place(Place::local(LocalId(0))),
        ty: TypeName::Int32,
        category: ValueCategory::Place,
        span: value.span,
    }));
    assert!(
        ownership::check_body(&hir)
            .unwrap_err()
            .message
            .contains("before initialization")
    );

    let mut hir = body(source, 0);
    hir.locals[0].properties.copy = KnownProperty::No;
    let Statement::Return { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Arithmetic { left, .. } = &mut value.kind else {
        panic!()
    };
    let ExpressionKind::Load(place) = &left.kind else {
        panic!()
    };
    left.kind = ExpressionKind::Move(place.clone());
    assert!(
        ownership::check_body(&hir)
            .unwrap_err()
            .message
            .contains("ownership was moved")
    );
}

#[test]
fn readonly_storage_blocks_mutation_in_ownership_analysis() {
    let mut hir = body(
        "Function Update(ByRef X As Integer) As Integer\nX = 2\nReturn X\nEnd Function",
        0,
    );
    hir.locals[0].storage = LocalStorage::BorrowedImmutable;
    assert!(
        ownership::check_body(&hir)
            .unwrap_err()
            .message
            .contains("ReadOnly reference")
    );
}

#[test]
fn scope_verifier_rejects_missing_unwind_scope() {
    let mut hir = body(
        "Function Use() As Integer\nIf True Then\nDim X As Integer = 1\nReturn X\nElse\nReturn 0\nEnd If\nEnd Function",
        0,
    );
    let Statement::If { then_body, .. } = &mut hir.statements[0] else {
        panic!()
    };
    let Statement::Return { exited_scopes, .. } = &mut then_body[1] else {
        panic!()
    };
    exited_scopes.remove(0);
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("scope exits")
    );
}

#[test]
fn scope_verifier_rejects_a_reference_after_its_scope_ends() {
    let mut hir = body(
        "Function Use(A As Boolean) As Integer\nIf A Then\nDim X As Integer = 1\nEnd If\nReturn 0\nEnd Function",
        0,
    );
    let Statement::Return { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    value.kind = ExpressionKind::Load(Box::new(Expression {
        kind: ExpressionKind::Place(Place::local(LocalId(1))),
        ty: TypeName::Int32,
        category: ValueCategory::Place,
        span: value.span,
    }));
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("outside its lexical scope")
    );
}

#[test]
fn a_returning_branch_does_not_poison_the_joined_ownership_state() {
    let mut hir = body(
        "Function Use(A As Boolean) As Integer\nDim X As Integer = 1\nIf A Then\nReturn X\nEnd If\nReturn X\nEnd Function",
        0,
    );
    hir.locals[1].properties.copy = KnownProperty::No;
    let Statement::If { then_body, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let Statement::Return { value, .. } = &mut then_body[0] else {
        panic!()
    };
    let ExpressionKind::Load(place) = &value.kind else {
        panic!()
    };
    value.kind = ExpressionKind::Move(place.clone());
    ownership::check_body(&hir).unwrap();
}

#[test]
fn internal_move_of_copy_value_preserves_source_and_noncopy_reinitializes() {
    let source = "Function Use() As Integer\nDim X As Integer = 1\nDim Y As Integer = X\nX = 2\nReturn X + Y\nEnd Function";
    let mut hir = body(source, 0);
    let Statement::Initialize { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Load(place) = &value.kind else {
        panic!()
    };
    value.kind = ExpressionKind::Move(place.clone());
    ownership::check_body(&hir).unwrap(); // Int32 is Copy.

    hir.locals[0].properties.copy = KnownProperty::No;
    ownership::check_body(&hir).unwrap(); // Store X reinitializes after move.
    hir.statements.remove(2);
    assert!(
        ownership::check_body(&hir)
            .unwrap_err()
            .message
            .contains("ownership was moved")
    );
}

#[test]
fn cleanup_report_skips_moved_owner_and_tracks_reinitialized_owner() {
    let source = "Function Use() As Integer\nDim X As Integer = 1\nDim Y As Integer = X\nReturn Y\nEnd Function";
    let mut hir = body(source, 0);
    hir.locals[0].properties.copy = KnownProperty::No;
    hir.locals[0].properties.requires_drop = KnownProperty::Yes;
    let Statement::Initialize { value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    let ExpressionKind::Load(place) = &value.kind else {
        panic!()
    };
    value.kind = ExpressionKind::Move(place.clone());
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(
        debug_hir::format_ownership_report(&report),
        "exit Return [0]:\n  local #1: NoDrop\n  local #0: SkipMoved\n"
    );
    assert_eq!(
        report.exits[0].locals,
        vec![
            (LocalId(1), ownership::CleanupAction::NoDrop),
            (LocalId(0), ownership::CleanupAction::SkipMoved),
        ]
    );

    let mut restored = body(
        "Function Use() As Integer\nDim X As Integer = 1\nDim Y As Integer = X\nX = 2\nReturn Y\nEnd Function",
        0,
    );
    restored.locals[0].properties.copy = KnownProperty::No;
    restored.locals[0].properties.requires_drop = KnownProperty::Yes;
    let Statement::Initialize { value, .. } = &mut restored.statements[1] else {
        panic!()
    };
    let ExpressionKind::Load(place) = &value.kind else {
        panic!()
    };
    value.kind = ExpressionKind::Move(place.clone());
    let report = ownership::analyze_body(&restored).unwrap();
    assert_eq!(
        report.replacements[0].old_value,
        ownership::CleanupAction::SkipMoved
    );
    assert_eq!(
        report.exits[0].locals[1],
        (LocalId(0), ownership::CleanupAction::Drop)
    );
}

#[test]
fn replacement_classifies_old_owner_and_rejects_self_move() {
    let mut hir = body(
        "Function Use() As Integer\nDim X As Integer = 1\nX = 2\nReturn X\nEnd Function",
        0,
    );
    hir.locals[0].properties.copy = KnownProperty::No;
    hir.locals[0].properties.requires_drop = KnownProperty::Yes;
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(
        report.replacements[0].old_value,
        ownership::CleanupAction::Drop
    );

    let Statement::Store { target, value, .. } = &mut hir.statements[1] else {
        panic!()
    };
    value.kind = ExpressionKind::Move(target.clone());
    assert!(
        ownership::analyze_body(&hir)
            .unwrap_err()
            .message
            .contains("Move of itself")
    );
}

#[test]
fn verifier_rejects_contradictory_copy_and_drop_properties() {
    let mut hir = body(
        "Function Use() As Integer\nDim X As Integer = 1\nReturn X\nEnd Function",
        0,
    );
    hir.locals[0].properties.requires_drop = KnownProperty::Yes;
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("Copy local")
    );
}

#[test]
fn normal_try_finally_has_ordered_typed_scopes() {
    let source = "Function Use() As Integer\nDim X As Integer = 0\nTry\nDim A As Integer = 1\nX = A\nFinally\nDim B As Integer = 2\nX += B\nEnd Try\nReturn X\nEnd Function";
    let hir = body(source, 0);
    let Statement::TryFinally {
        try_scope,
        finally_scope,
        ..
    } = &hir.statements[1]
    else {
        panic!()
    };
    assert_ne!(try_scope, finally_scope);
    let report = ownership::analyze_body(&hir).unwrap();
    let normal: Vec<_> = report
        .exits
        .iter()
        .filter(|exit| exit.kind == ownership::ExitKind::Normal)
        .collect();
    assert_eq!(
        normal[0].locals,
        vec![(LocalId(1), ownership::CleanupAction::NoDrop)]
    );
    assert_eq!(
        normal[1].locals,
        vec![(LocalId(2), ownership::CleanupAction::NoDrop)]
    );
    let printed = debug_hir::format_body(&hir);
    assert!(printed.find("try").unwrap() < printed.find("finally").unwrap());
}

#[test]
fn typed_try_finally_preserves_return_transfer_and_accepts_catch() {
    let source = "Function Use() As Integer\nTry\nReturn 1\nFinally\nDim X As Integer = 2\nEnd Try\nEnd Function";
    let hir = body(source, 0);
    let Statement::TryFinally {
        try_body,
        finally_scope,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    let Statement::Return { cleanup_chain, .. } = &try_body[0] else {
        panic!()
    };
    assert_eq!(cleanup_chain.len(), 1);
    assert_eq!(
        cleanup_chain[0].action,
        ScopeCleanup::FinallyRegion {
            finally_scope: *finally_scope
        }
    );
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(
        report.exits[0].handlers,
        vec![ownership::HandlerDecision::Finally(*finally_scope)]
    );
    let catch_source = "Function Use() As Integer\nTry\nDim X As Integer = 1\nCatch\nDim Y As Integer = 2\nEnd Try\nReturn 0\nEnd Function";
    let caught = body(catch_source, 0);
    assert!(matches!(
        caught.statements[0],
        Statement::TryCatch {
            finally_scope: None,
            ..
        }
    ));
}

#[test]
fn catch_local_is_initialized_and_return_runs_finally() {
    let source = "Function Use() As Integer\nTry\nReturn 1\nCatch Ex As Error\nReturn 2\nFinally\nDim X As Integer = 3\nEnd Try\nEnd Function";
    let hir = body(source, 0);
    let Statement::TryCatch {
        catch_scope,
        catch_local: Some(local),
        catch_body,
        finally_scope: Some(finally_scope),
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_eq!(hir.locals[local.0].scope, *catch_scope);
    assert_eq!(
        hir.locals[local.0].initial_state,
        valo_core::frontend::semantics::typed_hir::InitialState::Initialized
    );
    let Statement::Return { cleanup_chain, .. } = &catch_body[0] else {
        panic!()
    };
    assert_eq!(
        cleanup_chain[0].action,
        ScopeCleanup::FinallyRegion {
            finally_scope: *finally_scope
        }
    );
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(
        report
            .exits
            .iter()
            .filter(|exit| exit.kind == ownership::ExitKind::Return)
            .count(),
        2
    );
}

#[test]
fn nested_finally_and_using_have_one_ordered_return_chain() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(A As Resource, B As Resource) As Integer\nUsing A\nUsing B\nTry\nTry\nReturn 7\nFinally\nDim Inner As Integer = 1\nEnd Try\nFinally\nDim Outer As Integer = 2\nEnd Try\nEnd Using\nEnd Using\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    let transfer = report
        .exits
        .iter()
        .find(|exit| exit.kind == ownership::ExitKind::Return)
        .unwrap();
    assert_eq!(transfer.handlers.len(), 4);
    assert!(matches!(
        transfer.handlers[0],
        ownership::HandlerDecision::Finally(_)
    ));
    assert!(matches!(
        transfer.handlers[1],
        ownership::HandlerDecision::Finally(_)
    ));
    assert!(matches!(
        transfer.handlers[2],
        ownership::HandlerDecision::Dispose(_, _, ownership::CleanupAction::Dispose)
    ));
    assert!(matches!(
        transfer.handlers[3],
        ownership::HandlerDecision::Dispose(_, _, ownership::CleanupAction::Dispose)
    ));
    assert_ne!(transfer.disposals[0].0, transfer.disposals[1].0);
    assert!(debug_hir::format_body(&hir).contains("cleanup [CleanupStep"));
}

#[test]
fn loop_transfers_through_finally_keep_their_own_targets() {
    let source = "Function Use() As Integer\nDim I As Integer = 0\nFor I = 0 To 2\nTry\nIf I = 0 Then\nContinue For\nEnd If\nExit For\nFinally\nI += 1\nEnd Try\nNext\nReturn I\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    for kind in [
        ownership::ExitKind::ContinueLoop,
        ownership::ExitKind::ExitLoop,
    ] {
        let transfer = report.exits.iter().find(|exit| exit.kind == kind).unwrap();
        assert_eq!(transfer.handlers.len(), 1);
        assert!(matches!(
            transfer.handlers[0],
            ownership::HandlerDecision::Finally(_)
        ));
    }
}

#[test]
fn continue_from_using_inside_try_disposes_before_finally() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nDim I As Integer = 0\nFor I = 0 To 1\nTry\nUsing R\nContinue For\nEnd Using\nFinally\nI += 1\nEnd Try\nNext\nReturn I\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    let transfer = report
        .exits
        .iter()
        .find(|exit| exit.kind == ownership::ExitKind::ContinueLoop)
        .unwrap();
    assert_eq!(transfer.handlers.len(), 2);
    assert!(matches!(
        transfer.handlers[0],
        ownership::HandlerDecision::Dispose(_, _, ownership::CleanupAction::Dispose)
    ));
    assert!(matches!(
        transfer.handlers[1],
        ownership::HandlerDecision::Finally(_)
    ));
}

#[test]
fn return_inside_catch_runs_finally_then_using_once() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nUsing R\nTry\nDim X As Integer = 1\nCatch Ex As Error\nReturn 2\nFinally\nDim Y As Integer = 3\nEnd Try\nEnd Using\nReturn 0\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    let transfer = report
        .exits
        .iter()
        .find(|exit| exit.kind == ownership::ExitKind::Return && exit.handlers.len() == 2)
        .unwrap();
    assert!(matches!(
        transfer.handlers[0],
        ownership::HandlerDecision::Finally(_)
    ));
    assert!(matches!(
        transfer.handlers[1],
        ownership::HandlerDecision::Dispose(_, _, ownership::CleanupAction::Dispose)
    ));
    assert_eq!(transfer.disposals.len(), 1);
}

#[test]
fn transfer_out_of_finally_remains_rejected_by_typed_hir() {
    let source =
        "Function Use() As Integer\nTry\nReturn 1\nFinally\nReturn 2\nEnd Try\nEnd Function";
    let result = parse_source(source).and_then(|program| lower_function_body(&program, 0));
    assert!(result.is_err());
}

#[test]
fn verifier_rejects_missing_or_duplicated_transfer_cleanup() {
    let source = "Function Use() As Integer\nTry\nReturn 1\nFinally\nDim X As Integer = 2\nEnd Try\nEnd Function";
    let mut hir = body(source, 0);
    let Statement::TryFinally { try_body, .. } = &mut hir.statements[0] else {
        panic!()
    };
    let Statement::Return { cleanup_chain, .. } = &mut try_body[0] else {
        panic!()
    };
    cleanup_chain.push(cleanup_chain[0].clone());
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("incorrect cleanup chain")
    );
}

#[test]
fn verifier_rejects_uninitialized_catch_binding() {
    let source = "Function Use() As Integer\nTry\nReturn 1\nCatch Ex As Error\nReturn 2\nEnd Try\nEnd Function";
    let mut hir = body(source, 0);
    let Statement::TryCatch {
        catch_local: Some(local),
        ..
    } = hir.statements[0]
    else {
        panic!()
    };
    hir.locals[local.0].initial_state =
        valo_core::frontend::semantics::typed_hir::InitialState::Uninitialized;
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("Catch local is not initialized")
    );
}

#[test]
fn verifier_rejects_mismatched_try_handler() {
    let source = "Function Use() As Integer\nTry\nReturn 1\nFinally\nDim X As Integer = 2\nEnd Try\nEnd Function";
    let mut hir = body(source, 0);
    let Statement::TryFinally { try_scope, .. } = hir.statements[0] else {
        panic!()
    };
    hir.scopes[try_scope.0].cleanup.clear();
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("no matching Finally handler")
    );
}

#[test]
fn verifier_rejects_try_finally_with_shared_scope_identity() {
    let mut hir = body(
        "Function Use() As Integer\nTry\nDim A As Integer = 1\nFinally\nDim B As Integer = 2\nEnd Try\nReturn 0\nEnd Function",
        0,
    );
    let Statement::TryFinally {
        try_scope,
        finally_scope,
        ..
    } = &mut hir.statements[0]
    else {
        panic!()
    };
    *finally_scope = *try_scope;
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("cannot share a scope")
    );
}

#[test]
fn existing_disposable_using_target_has_resolved_hir_cleanup() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nUsing R\nDim X As Integer = 1\nEnd Using\nReturn 0\nEnd Function";
    let hir = body(source, 0);
    let Statement::UsingDispose {
        resource,
        method,
        body_scope,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_eq!(
        hir.scopes[body_scope.0].cleanup,
        vec![ScopeCleanup::ExplicitDispose {
            resource: *resource,
            method: *method
        }]
    );
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(
        report.exits[0].disposals,
        vec![(*resource, *method, ownership::CleanupAction::Dispose)]
    );
    assert!(debug_hir::format_ownership_report(&report).contains("dispose local"));
}

#[test]
fn return_from_existing_disposable_using_target_keeps_cleanup() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nUsing R\nReturn 7\nEnd Using\nEnd Function";
    let hir = body(source, 0);
    let Statement::UsingDispose {
        resource,
        method,
        body: using_body,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    let Statement::Return { exited_scopes, .. } = &using_body[1] else {
        panic!()
    };
    assert_eq!(exited_scopes.len(), 2);
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(report.exits[0].kind, ownership::ExitKind::Return);
    assert_eq!(
        report.exits[0].disposals,
        vec![(*resource, *method, ownership::CleanupAction::Dispose)]
    );
}

#[test]
fn nested_disposable_using_targets_cleanup_inner_first() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(A As Resource, B As Resource) As Integer\nUsing A\nUsing B\nReturn 1\nEnd Using\nEnd Using\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    let disposals = &report.exits[0].disposals;
    assert_eq!(disposals.len(), 2);
    assert_eq!(disposals[0].0, LocalId(3));
    assert_eq!(disposals[1].0, LocalId(2));
    assert!(
        disposals
            .iter()
            .all(|entry| entry.2 == ownership::CleanupAction::Dispose)
    );
}

#[test]
fn loop_jumps_from_using_target_include_dispose_obligation() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nDim I As Integer = 0\nFor I = 0 To 2\nUsing R\nIf I = 0 Then\nContinue For\nEnd If\nExit For\nEnd Using\nNext\nReturn I\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    for kind in [
        ownership::ExitKind::ContinueLoop,
        ownership::ExitKind::ExitLoop,
    ] {
        assert!(report.exits.iter().any(|exit| exit.kind == kind
            && exit.disposals.len() == 1
            && exit.disposals[0].2 == ownership::CleanupAction::Dispose));
    }
}

#[test]
fn normal_try_finally_inside_using_disposes_after_finally() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nUsing R\nTry\nDim X As Integer = 1\nFinally\nDim Y As Integer = 2\nEnd Try\nEnd Using\nReturn 0\nEnd Function";
    let hir = body(source, 0);
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(report.exits[0].kind, ownership::ExitKind::Normal);
    assert_eq!(report.exits[1].kind, ownership::ExitKind::Normal);
    assert!(report.exits[0].disposals.is_empty());
    assert!(report.exits[1].disposals.is_empty());
    assert_eq!(
        report.exits[2].disposals[0].2,
        ownership::CleanupAction::Dispose
    );
}

#[test]
fn verifier_rejects_using_without_matching_scope_cleanup() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(R As Resource) As Integer\nUsing R\nReturn 1\nEnd Using\nEnd Function";
    let mut hir = body(source, 0);
    let Statement::UsingDispose { body_scope, .. } = &hir.statements[0] else {
        panic!()
    };
    let scope = *body_scope;
    hir.scopes[scope.0].cleanup.clear();
    assert!(
        verify_hir::verify_body(&hir)
            .unwrap_err()
            .message
            .contains("no matching Dispose cleanup")
    );
}

#[test]
fn typed_hir_reports_using_declaration_limit_explicitly() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use() As Integer\nUsing R As New Resource()\nReturn 1\nEnd Using\nEnd Function";
    let error = lower_function_body(&parse_source(source).unwrap(), 0).unwrap_err();
    assert!(
        error
            .message
            .contains("Using declarations requiring construction")
    );
}

#[test]
fn existing_using_declaration_with_initializer_has_typed_dispose_scope() {
    let source = "Class Resource\nPublic Sub Dispose()\nEnd Sub\nEnd Class\nFunction Use(Existing As Resource) As Integer\nUsing R As Resource = Existing\nReturn 3\nEnd Using\nEnd Function";
    let hir = body(source, 0);
    let Statement::UsingDispose {
        resource,
        body_scope,
        body,
        ..
    } = &hir.statements[0]
    else {
        panic!()
    };
    assert_eq!(hir.locals[resource.0].name, "R");
    assert_eq!(hir.locals[resource.0].scope, *body_scope);
    assert!(matches!(body[0], Statement::Initialize { target, .. } if target == *resource));
    let report = ownership::analyze_body(&hir).unwrap();
    assert_eq!(report.exits[0].disposals.len(), 1);
}

#[test]
fn normal_branch_and_loop_scope_exits_have_cleanup_reports() {
    let hir = body(
        "Function Use() As Integer\nDim X As Integer = 0\nIf True Then\nDim BranchValue As Integer = 1\nEnd If\nWhile X < 1\nDim LoopValue As Integer = 2\nX = 1\nWend\nReturn X\nEnd Function",
        0,
    );
    let report = ownership::analyze_body(&hir).unwrap();
    assert!(
        report
            .exits
            .iter()
            .any(|exit| exit.kind == ownership::ExitKind::Normal
                && exit.locals == vec![(LocalId(1), ownership::CleanupAction::NoDrop)])
    );
    assert!(
        report
            .exits
            .iter()
            .any(|exit| exit.kind == ownership::ExitKind::Normal
                && exit.locals == vec![(LocalId(2), ownership::CleanupAction::NoDrop)])
    );
}
