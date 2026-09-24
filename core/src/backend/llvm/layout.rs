//! Fixed-array shapes are inferred from resolved MIR initializers, never from source syntax.
//! Array TypeName currently records the element type but not the fixed bound.
use crate::frontend::type_model::TypeName;
use crate::mir::ir::{self as m, InstructionKind};

use super::BackendError;

pub(super) struct ArrayShapes {
    pub locals: Vec<Option<usize>>,
    pub temps: Vec<Option<usize>>,
}

impl ArrayShapes {
    pub fn analyze(function: &m::Function) -> Result<Self, BackendError> {
        let mut shapes = Self {
            locals: vec![None; function.locals.len()],
            temps: vec![None; function.temps.len()],
        };
        // Stores and snapshots can be connected through a loop in the CFG.
        // Monotone propagation of fixed lengths converges independently of block order.
        let limit = function.locals.len() + function.temps.len() + 1;
        for _ in 0..limit {
            let mut changed = false;
            for block in &function.blocks {
                for instruction in &block.instructions {
                    let result = instruction.result.map(|id| id.0);
                    match &instruction.kind {
                        InstructionKind::ArrayInit { upper } => {
                            let length = upper
                                .checked_add(1)
                                .and_then(|n| usize::try_from(n).ok())
                                .ok_or_else(|| {
                                    BackendError::new(
                                        "native eligibility",
                                        "fixed array bound is too large",
                                    )
                                })?;
                            if length > 1_000_000 {
                                return Err(BackendError::new(
                                    "native eligibility",
                                    "fixed array exceeds current native stack limit",
                                ));
                            }
                            changed |= set(&mut shapes.temps[result.expect("verified")], length)?;
                        }
                        InstructionKind::SnapshotArray(place) | InstructionKind::Load(place)
                            if matches!(place.ty, TypeName::Array(_))
                                && place.projections.is_empty() =>
                        {
                            if let Some(length) = shapes.locals[place.root.0] {
                                changed |=
                                    set(&mut shapes.temps[result.expect("verified")], length)?;
                            }
                        }
                        InstructionKind::Store { place, value }
                            if matches!(place.ty, TypeName::Array(_))
                                && place.projections.is_empty() =>
                        {
                            if let Some(length) = shapes.temps[value.0] {
                                changed |= set(&mut shapes.locals[place.root.0], length)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for (index, local) in function.locals.iter().enumerate() {
            if matches!(local.ty, TypeName::Array(_)) && shapes.locals[index].is_none() {
                return Err(BackendError::new(
                    "native eligibility",
                    "fixed array local has no resolved length",
                ));
            }
        }
        for (index, ty) in function.temps.iter().enumerate() {
            if matches!(ty, TypeName::Array(_)) && shapes.temps[index].is_none() {
                return Err(BackendError::new(
                    "native eligibility",
                    "fixed array temporary has no resolved length",
                ));
            }
        }
        Ok(shapes)
    }
}

fn set(slot: &mut Option<usize>, length: usize) -> Result<bool, BackendError> {
    match slot {
        Some(existing) if *existing != length => Err(BackendError::new(
            "native eligibility",
            "fixed array is assigned incompatible lengths",
        )),
        Some(_) => Ok(false),
        None => {
            *slot = Some(length);
            Ok(true)
        }
    }
}
