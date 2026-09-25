//! Expands HIR scope-exit obligations into explicit MIR Drop operations.
//! Uncertain ownership needs future drop flags; it is rejected, never dropped
//! unconditionally or silently leaked.
use crate::frontend::semantics::type_properties::KnownProperty;
use crate::frontend::semantics::typed_hir::LocalStorage;
use crate::frontend::type_model::TypeName;

use super::analysis::{cfg::Cfg, dataflow::solve_forward};
use super::ir::{Function, Instruction, InstructionKind, Place};

const UNINITIALIZED: u8 = 1;
const AVAILABLE: u8 = 2;
const MOVED: u8 = 4;
const DROPPED: u8 = 8;

fn transfer(state: &mut [u8], kind: &InstructionKind, function: &Function) {
    match kind {
        InstructionKind::Store { place, .. } | InstructionKind::Replace { place, .. }
            if place.projections.is_empty() =>
        {
            state[place.root.0] = AVAILABLE
        }
        InstructionKind::Move(place)
            if function.locals[place.root.0].properties.copy == KnownProperty::No =>
        {
            state[place.root.0] = MOVED;
        }
        InstructionKind::Drop(place) => state[place.root.0] = DROPPED,
        InstructionKind::DropCandidate(id)
            if function.locals[id.0].properties.requires_drop == KnownProperty::Yes =>
        {
            let previous = state[id.0];
            state[id.0] = (if previous & AVAILABLE != 0 {
                DROPPED
            } else {
                0
            }) | (previous & !AVAILABLE);
        }
        _ => {}
    }
}

pub fn elaborate(function: &mut Function) -> Result<(), String> {
    let cfg = Cfg::build(function)?;
    let initial = function
        .locals
        .iter()
        .map(|local| {
            if local.parameter_index.is_some() {
                AVAILABLE
            } else {
                UNINITIALIZED
            }
        })
        .collect();
    let states = solve_forward(
        function,
        &cfg,
        initial,
        |left: &Vec<u8>, right: &Vec<u8>| left.iter().zip(right).map(|(a, b)| a | b).collect(),
        |block, input| {
            let mut state = input.clone();
            for instruction in &block.instructions {
                transfer(&mut state, &instruction.kind, function);
            }
            state
        },
    );
    let snapshot = function.clone();
    for block in &mut function.blocks {
        let Some(mut state) = states.entry[block.id.0].clone() else {
            block
                .instructions
                .retain(|ins| !matches!(ins.kind, InstructionKind::DropCandidate(_)));
            continue;
        };
        let mut lowered = Vec::with_capacity(block.instructions.len());
        for instruction in block.instructions.drain(..) {
            let InstructionKind::DropCandidate(id) = instruction.kind else {
                transfer(&mut state, &instruction.kind, &snapshot);
                lowered.push(instruction);
                continue;
            };
            let local = snapshot
                .locals
                .get(id.0)
                .ok_or("Drop candidate has invalid local")?;
            if local.storage != LocalStorage::Value {
                return Err("Drop candidate must own its local".into());
            }
            let candidate = InstructionKind::DropCandidate(id);
            match local.properties.requires_drop {
                KnownProperty::No | KnownProperty::Unknown => {}
                KnownProperty::Yes if local.ty != TypeName::String => {
                    return Err(format!(
                        "native Drop elaboration has no contract for {:?}",
                        local.ty
                    ));
                }
                KnownProperty::Yes if state[id.0] == AVAILABLE => lowered.push(Instruction {
                    result: None,
                    kind: InstructionKind::Drop(Place {
                        root: id,
                        projections: Vec::new(),
                        ty: local.ty.clone(),
                    }),
                    span: instruction.span,
                }),
                KnownProperty::Yes if state[id.0] & AVAILABLE == 0 => {}
                KnownProperty::Yes => {
                    return Err(format!(
                        "local #{} needs conditional Drop flags on this path",
                        id.0
                    ));
                }
            }
            transfer(&mut state, &candidate, &snapshot);
        }
        block.instructions = lowered;
    }
    Ok(())
}
