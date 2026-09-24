//! Stable, human-readable MIR dump for tests and compiler development.
use std::fmt::Write;

use super::ir::*;

pub fn format_function(function: &Function) -> String {
    let mut output = String::new();
    let parameters = function
        .locals
        .iter()
        .filter_map(|local| local.parameter_index.map(|index| (index, local)))
        .collect::<Vec<_>>();
    let args = parameters
        .iter()
        .map(|(_, local)| format!("${}: {:?}", local.id.0, local.ty))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(
        output,
        "fn #{} {}({args}) -> {:?}",
        function.id.0, function.name, function.return_type
    )
    .unwrap();
    for block in &function.blocks {
        writeln!(
            output,
            "bb{}: ; {}",
            block.id.0,
            block.label.as_deref().unwrap_or("block")
        )
        .unwrap();
        for instruction in &block.instructions {
            if let Some(result) = instruction.result {
                write!(output, "  %{}: {:?} = ", result.0, function.temps[result.0]).unwrap();
            } else {
                write!(output, "  ").unwrap();
            }
            writeln!(output, "{}", format_instruction(&instruction.kind)).unwrap();
        }
        if let Some(terminator) = &block.terminator {
            writeln!(output, "  {}", format_terminator(&terminator.kind)).unwrap();
        } else {
            writeln!(output, "  <missing terminator>").unwrap();
        }
    }
    output
}

fn format_instruction(kind: &InstructionKind) -> String {
    match kind {
        InstructionKind::Const(value) => format!("const {value:?}"),
        InstructionKind::TupleInit(values) => format!("tuple.init {values:?}"),
        InstructionKind::ArrayInit { upper } => format!("array.init 0..={upper}"),
        InstructionKind::ArrayLen(array) => format!("array.len {}", format_place(array)),
        InstructionKind::SnapshotArray(array) => format!("array.snapshot {}", format_place(array)),
        InstructionKind::Load(place) => format!("load {}", format_place(place)),
        InstructionKind::Move(place) => format!("move {}", format_place(place)),
        InstructionKind::Drop(place) => format!("drop {}", format_place(place)),
        InstructionKind::BorrowStart { id, kind, place } => {
            format!("borrow.start #{} {kind:?} {}", id.0, format_place(place))
        }
        InstructionKind::EndBorrow(id) => format!("borrow.end #{}", id.0),
        InstructionKind::Store { place, value } => {
            format!("store {}, %{}", format_place(place), value.0)
        }
        InstructionKind::Arithmetic { op, left, right } => {
            format!("{op:?} %{}, %{}", left.0, right.0)
        }
        InstructionKind::Compare { op, left, right } => {
            format!("cmp.{op:?} %{}, %{}", left.0, right.0)
        }
        InstructionKind::Cast { value, conversion } => format!("cast.{conversion:?} %{}", value.0),
        InstructionKind::Call {
            target, arguments, ..
        } => {
            let args = arguments
                .iter()
                .map(|arg| match arg {
                    CallArgument::Value(id) => format!("%{}", id.0),
                    CallArgument::Place { mode, place } => {
                        format!("{mode:?} {}", format_place(place))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            match target {
                CallTarget::Function(id) => format!("call fn#{}({args})", id.0),
                CallTarget::Dispose(id) => format!("call dispose#{}({args})", id.0),
            }
        }
    }
}

fn format_place(place: &Place) -> String {
    let mut output = format!("${}", place.root.0);
    for projection in &place.projections {
        match projection {
            Projection::Field(id) => write!(output, ".field#{}", id.0).unwrap(),
            Projection::TupleField(index) => write!(output, ".tuple#{index}").unwrap(),
            Projection::Index(index) => write!(output, "[%{}]", index.0).unwrap(),
        }
    }
    output
}

fn format_terminator(kind: &TerminatorKind) -> String {
    match kind {
        TerminatorKind::Goto(id) => format!("goto bb{}", id.0),
        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => format!(
            "branch %{}, bb{}, bb{}",
            condition.0, then_block.0, else_block.0
        ),
        TerminatorKind::Return(value) => format!("return %{}", value.0),
        TerminatorKind::ReturnVoid => "return void".into(),
        TerminatorKind::Trap(message) => format!("trap {message:?}"),
        TerminatorKind::Unreachable => "unreachable".into(),
    }
}
