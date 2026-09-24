use valo_core::frontend::semantics::type_properties::{KnownProperty, TypeProperties};
use valo_core::mir::{
    analysis::{cfg::Cfg, ownership},
    ir, lower_body, verify,
};
use valo_core::semantics::lower_function_body;
use valo_core::{TypeName, parse_source};

fn fixture() -> ir::Function {
    let program =
        parse_source("Function F(A As Integer) As Integer\nReturn A\nEnd Function").unwrap();
    let hir = lower_function_body(&program, 0).unwrap();
    lower_body(&hir).unwrap()
}

fn place(id: usize) -> ir::Place {
    ir::Place {
        root: ir::LocalId(id),
        projections: vec![],
        ty: TypeName::Int32,
    }
}

fn add_local(function: &mut ir::Function, properties: TypeProperties) {
    let id = ir::LocalId(function.locals.len());
    function.locals.push(ir::Local {
        id,
        ty: TypeName::Int32,
        properties,
        storage: valo_core::semantics::typed_hir::LocalStorage::Value,
        parameter_index: None,
        span: function.span,
    });
}

fn instruction(
    span: valo_core::Span,
    result: Option<usize>,
    kind: ir::InstructionKind,
) -> ir::Instruction {
    ir::Instruction {
        result: result.map(ir::TempId),
        kind,
        span,
    }
}

fn block(
    function: &ir::Function,
    id: usize,
    instructions: Vec<ir::Instruction>,
    term: ir::TerminatorKind,
) -> ir::BasicBlock {
    ir::BasicBlock {
        id: ir::BlockId(id),
        label: None,
        instructions,
        terminator: Some(ir::Terminator {
            kind: term,
            span: function.span,
        }),
    }
}

fn plain() -> TypeProperties {
    TypeProperties {
        copy: KnownProperty::Yes,
        requires_drop: KnownProperty::No,
    }
}
fn resource() -> TypeProperties {
    TypeProperties {
        copy: KnownProperty::No,
        requires_drop: KnownProperty::Yes,
    }
}

#[test]
fn cfg_reports_stable_edges_reachability_and_unreachable_blocks() {
    let mut f = fixture();
    f.temps = vec![TypeName::Boolean, TypeName::Int32];
    f.blocks = vec![
        block(
            &f,
            0,
            vec![instruction(
                f.span,
                Some(0),
                ir::InstructionKind::Const(ir::Constant::Boolean(true)),
            )],
            ir::TerminatorKind::Branch {
                condition: ir::TempId(0),
                then_block: ir::BlockId(1),
                else_block: ir::BlockId(2),
            },
        ),
        block(&f, 1, vec![], ir::TerminatorKind::Goto(ir::BlockId(2))),
        block(
            &f,
            2,
            vec![instruction(
                f.span,
                Some(1),
                ir::InstructionKind::Load(place(0)),
            )],
            ir::TerminatorKind::Return(ir::TempId(1)),
        ),
        block(&f, 3, vec![], ir::TerminatorKind::Unreachable),
    ];
    let cfg = Cfg::build(&f).unwrap();
    assert_eq!(cfg.successors[0], vec![ir::BlockId(1), ir::BlockId(2)]);
    assert_eq!(cfg.predecessors[2], vec![ir::BlockId(0), ir::BlockId(1)]);
    assert_eq!(cfg.reachable, vec![true, true, true, false]);
    assert_eq!(cfg.reverse_postorder.first(), Some(&ir::BlockId(0)));
    assert!(ownership::analyze(&f).is_ok());
}

fn diamond(
    then_ops: Vec<ir::InstructionKind>,
    else_ops: Vec<ir::InstructionKind>,
    properties: TypeProperties,
) -> ir::Function {
    let mut f = fixture();
    add_local(&mut f, properties);
    f.temps = vec![TypeName::Boolean, TypeName::Int32, TypeName::Int32];
    let store = |f: &ir::Function| {
        instruction(
            f.span,
            None,
            ir::InstructionKind::Store {
                place: place(1),
                value: ir::TempId(1),
            },
        )
    };
    let then_instructions = then_ops
        .into_iter()
        .map(|kind| {
            instruction(
                f.span,
                if matches!(kind, ir::InstructionKind::Move(_)) {
                    Some(2)
                } else {
                    None
                },
                kind,
            )
        })
        .collect();
    let else_instructions = else_ops
        .into_iter()
        .map(|kind| {
            instruction(
                f.span,
                if matches!(kind, ir::InstructionKind::Move(_)) {
                    Some(2)
                } else {
                    None
                },
                kind,
            )
        })
        .collect();
    f.blocks = vec![
        block(
            &f,
            0,
            vec![
                instruction(
                    f.span,
                    Some(0),
                    ir::InstructionKind::Const(ir::Constant::Boolean(true)),
                ),
                instruction(f.span, Some(1), ir::InstructionKind::Load(place(0))),
                store(&f),
            ],
            ir::TerminatorKind::Branch {
                condition: ir::TempId(0),
                then_block: ir::BlockId(1),
                else_block: ir::BlockId(2),
            },
        ),
        block(
            &f,
            1,
            then_instructions,
            ir::TerminatorKind::Goto(ir::BlockId(3)),
        ),
        block(
            &f,
            2,
            else_instructions,
            ir::TerminatorKind::Goto(ir::BlockId(3)),
        ),
        block(&f, 3, vec![], ir::TerminatorKind::Return(ir::TempId(1))),
    ];
    f
}

#[test]
fn move_on_one_path_becomes_maybe_unavailable_at_join() {
    let f = diamond(
        vec![ir::InstructionKind::Move(place(1))],
        vec![],
        resource(),
    );
    let report = ownership::analyze(&f).unwrap();
    assert_eq!(
        ownership::local_state(&report, ir::BlockId(3), ir::LocalId(1)),
        Some("MaybeUnavailable")
    );
    let mut bad = f;
    bad.temps.push(TypeName::Int32);
    bad.blocks[3].instructions.push(instruction(
        bad.span,
        Some(3),
        ir::InstructionKind::Load(place(1)),
    ));
    assert!(
        ownership::analyze(&bad)
            .unwrap_err()
            .contains("unavailable on some path")
    );
}

#[test]
fn reinitialization_after_move_and_copy_move_are_available() {
    let mut f = diamond(
        vec![
            ir::InstructionKind::Move(place(1)),
            ir::InstructionKind::Store {
                place: place(1),
                value: ir::TempId(1),
            },
        ],
        vec![],
        resource(),
    );
    assert_eq!(
        ownership::local_state(
            &ownership::analyze(&f).unwrap(),
            ir::BlockId(3),
            ir::LocalId(1)
        ),
        Some("Available")
    );
    f.locals[1].properties = plain();
    assert_eq!(
        ownership::local_state(
            &ownership::analyze(&f).unwrap(),
            ir::BlockId(3),
            ir::LocalId(1)
        ),
        Some("Available")
    );
}

#[test]
fn drop_is_distinct_from_dispose_and_cannot_double_drop_or_drop_after_move() {
    let mut f = fixture();
    add_local(&mut f, resource());
    f.blocks[0].instructions.insert(
        1,
        instruction(
            f.span,
            None,
            ir::InstructionKind::Store {
                place: place(1),
                value: ir::TempId(0),
            },
        ),
    );
    // The initial return fixture defines %0 by loading its parameter.
    f.blocks[0].instructions.insert(
        2,
        instruction(f.span, None, ir::InstructionKind::Drop(place(1))),
    );
    assert!(ownership::analyze(&f).is_ok());
    f.blocks[0].instructions.insert(
        3,
        instruction(f.span, None, ir::InstructionKind::Drop(place(1))),
    );
    assert!(ownership::analyze(&f).unwrap_err().contains("dropped"));
    f.temps.push(TypeName::Int32);
    f.blocks[0].instructions[2] = instruction(f.span, Some(1), ir::InstructionKind::Move(place(1)));
    assert!(ownership::analyze(&f).unwrap_err().contains("moved"));
}

#[test]
fn active_borrows_conflict_but_end_borrow_releases_place() {
    let mut f = fixture();
    let start = ir::InstructionKind::BorrowStart {
        id: ir::BorrowId(0),
        kind: ir::BorrowKind::Shared,
        place: place(0),
    };
    f.blocks[0]
        .instructions
        .insert(0, instruction(f.span, None, start));
    f.blocks[0].instructions.insert(
        1,
        instruction(f.span, None, ir::InstructionKind::Move(place(0))),
    );
    f.locals[0].properties = resource();
    f.temps.push(TypeName::Int32);
    f.blocks[0].instructions[1].result = Some(ir::TempId(1));
    assert!(
        ownership::analyze(&f)
            .unwrap_err()
            .contains("active borrow")
    );
    f.blocks[0].instructions.insert(
        1,
        instruction(
            f.span,
            None,
            ir::InstructionKind::EndBorrow(ir::BorrowId(0)),
        ),
    );
    // Return uses the prior parameter load, which now occurs after Move.
    assert!(ownership::analyze(&f).unwrap_err().contains("moved"));
}

#[test]
fn verifier_rejects_temp_defined_only_on_one_branch() {
    let mut f = fixture();
    f.temps = vec![TypeName::Boolean, TypeName::Int32];
    f.blocks = vec![
        block(
            &f,
            0,
            vec![instruction(
                f.span,
                Some(0),
                ir::InstructionKind::Const(ir::Constant::Boolean(true)),
            )],
            ir::TerminatorKind::Branch {
                condition: ir::TempId(0),
                then_block: ir::BlockId(1),
                else_block: ir::BlockId(2),
            },
        ),
        block(
            &f,
            1,
            vec![instruction(
                f.span,
                Some(1),
                ir::InstructionKind::Load(place(0)),
            )],
            ir::TerminatorKind::Goto(ir::BlockId(3)),
        ),
        block(&f, 2, vec![], ir::TerminatorKind::Goto(ir::BlockId(3))),
        block(&f, 3, vec![], ir::TerminatorKind::Return(ir::TempId(1))),
    ];
    assert!(
        verify::verify(&f)
            .unwrap_err()
            .contains("before definition")
    );
}

#[test]
fn lowered_branch_and_loop_are_dataflow_analyzable() {
    for source in [
        "Function F(A As Integer) As Integer\nDim X As Integer = 0\nIf A > 0 Then\nX = A\nElse\nX = 1\nEnd If\nReturn X\nEnd Function",
        "Function F(A As Integer) As Integer\nDim X As Integer = 0\nWhile X < A\nX += 1\nWend\nReturn X\nEnd Function",
    ] {
        let program = parse_source(source).unwrap();
        let hir = lower_function_body(&program, 0).unwrap();
        let mir = lower_body(&hir).unwrap();
        ownership::analyze(&mir).unwrap();
    }
}

#[test]
fn definite_initialization_requires_every_branch() {
    let mut f = fixture();
    add_local(&mut f, plain());
    f.temps = vec![TypeName::Boolean, TypeName::Int32, TypeName::Int32];
    f.blocks = vec![
        block(
            &f,
            0,
            vec![
                instruction(
                    f.span,
                    Some(0),
                    ir::InstructionKind::Const(ir::Constant::Boolean(true)),
                ),
                instruction(f.span, Some(1), ir::InstructionKind::Load(place(0))),
            ],
            ir::TerminatorKind::Branch {
                condition: ir::TempId(0),
                then_block: ir::BlockId(1),
                else_block: ir::BlockId(2),
            },
        ),
        block(
            &f,
            1,
            vec![instruction(
                f.span,
                None,
                ir::InstructionKind::Store {
                    place: place(1),
                    value: ir::TempId(1),
                },
            )],
            ir::TerminatorKind::Goto(ir::BlockId(3)),
        ),
        block(&f, 2, vec![], ir::TerminatorKind::Goto(ir::BlockId(3))),
        block(
            &f,
            3,
            vec![instruction(
                f.span,
                Some(2),
                ir::InstructionKind::Load(place(1)),
            )],
            ir::TerminatorKind::Return(ir::TempId(2)),
        ),
    ];
    assert!(
        ownership::analyze(&f)
            .unwrap_err()
            .contains("unavailable on some path")
    );
    f.blocks[2].instructions.push(instruction(
        f.span,
        None,
        ir::InstructionKind::Store {
            place: place(1),
            value: ir::TempId(1),
        },
    ));
    assert_eq!(
        ownership::local_state(
            &ownership::analyze(&f).unwrap(),
            ir::BlockId(3),
            ir::LocalId(1)
        ),
        Some("Available")
    );
}

#[test]
fn droppable_replacement_requires_explicit_drop_after_rhs() {
    let mut f = fixture();
    add_local(&mut f, resource());
    f.blocks[0].instructions.push(instruction(
        f.span,
        None,
        ir::InstructionKind::Store {
            place: place(1),
            value: ir::TempId(0),
        },
    ));
    f.blocks[0].instructions.push(instruction(
        f.span,
        None,
        ir::InstructionKind::Store {
            place: place(1),
            value: ir::TempId(0),
        },
    ));
    assert!(
        ownership::analyze(&f)
            .unwrap_err()
            .contains("requires Drop")
    );
    f.blocks[0].instructions.insert(
        2,
        instruction(f.span, None, ir::InstructionKind::Drop(place(1))),
    );
    assert!(ownership::analyze(&f).is_ok());
}

#[test]
fn shared_borrows_coexist_but_mutable_overlap_conflicts() {
    let mut f = fixture();
    let start = |id, kind| {
        instruction(
            f.span,
            None,
            ir::InstructionKind::BorrowStart {
                id: ir::BorrowId(id),
                kind,
                place: place(0),
            },
        )
    };
    let first = start(0, ir::BorrowKind::Shared);
    let second = start(1, ir::BorrowKind::Shared);
    f.blocks[0].instructions.splice(
        0..0,
        [
            first,
            second,
            instruction(
                f.span,
                None,
                ir::InstructionKind::EndBorrow(ir::BorrowId(1)),
            ),
            instruction(
                f.span,
                None,
                ir::InstructionKind::EndBorrow(ir::BorrowId(0)),
            ),
        ],
    );
    assert!(ownership::analyze(&f).is_ok());
    if let ir::InstructionKind::BorrowStart { kind, .. } = &mut f.blocks[0].instructions[1].kind {
        *kind = ir::BorrowKind::Mutable;
    }
    assert!(
        ownership::analyze(&f)
            .unwrap_err()
            .contains("active borrow")
    );
}

#[test]
fn place_overlap_is_conservative_for_fields_and_indices() {
    let mut f = fixture();
    let whole = place(0);
    let field = |id| ir::Place {
        root: ir::LocalId(0),
        projections: vec![ir::Projection::Field(
            valo_core::semantics::typed_hir::FieldId(id),
        )],
        ty: TypeName::Int32,
    };
    assert!(ownership::may_overlap(&f, &whole, &field(0)));
    assert!(!ownership::may_overlap(&f, &field(0), &field(1)));
    f.temps = vec![TypeName::Int32; 3];
    f.blocks[0].instructions = vec![
        instruction(
            f.span,
            Some(0),
            ir::InstructionKind::Const(ir::Constant::Integer(0)),
        ),
        instruction(
            f.span,
            Some(1),
            ir::InstructionKind::Const(ir::Constant::Integer(1)),
        ),
        instruction(f.span, Some(2), ir::InstructionKind::Load(whole)),
    ];
    let indexed = |id| ir::Place {
        root: ir::LocalId(0),
        projections: vec![ir::Projection::Index(ir::TempId(id))],
        ty: TypeName::Int32,
    };
    assert!(!ownership::may_overlap(&f, &indexed(0), &indexed(1)));
    assert!(ownership::may_overlap(&f, &indexed(0), &indexed(2)));
}

#[test]
fn resolved_call_borrows_end_at_the_call() {
    let source = "Function Inspect(ByRef ReadOnly A As Integer) As Integer\nReturn A\nEnd Function\nFunction Modify(ByRef A As Integer) As Integer\nA = 2\nReturn A\nEnd Function\nFunction F(X As Integer) As Integer\nReturn Inspect(X) + Modify(X)\nEnd Function";
    let program = parse_source(source).unwrap();
    let hir = lower_function_body(&program, 2).unwrap();
    let mir = lower_body(&hir).unwrap();
    assert!(ownership::analyze(&mir).is_ok());
}
