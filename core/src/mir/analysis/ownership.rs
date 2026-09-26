//! Conservative, path-sensitive state for whole MIR locals. Projected ownership
//! and escaping references require later move-path and lifetime passes.
use std::collections::{BTreeMap, BTreeSet};

use super::{
    cfg::Cfg,
    dataflow::{ForwardResult, solve_forward},
};
use crate::frontend::semantics::type_properties::KnownProperty;
use crate::frontend::semantics::typed_hir::ArgumentMode;
use crate::mir::ir::{
    BorrowId, BorrowKind, CallArgument, Constant, Function, InstructionKind, LocalId, Place,
    Projection, TempId, TerminatorKind,
};

const UNINITIALIZED: u8 = 1;
const AVAILABLE: u8 = 2;
const MOVED: u8 = 4;
const DROPPED: u8 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Bitwise union of states observed along reachable paths.
    pub locals: Vec<u8>,
    /// A borrow remains live until its explicit EndBorrow. Call arguments are
    /// checked together and end at the call instruction.
    pub borrows: BTreeSet<BorrowId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub cfg: Cfg,
    pub states: ForwardResult<State>,
}

pub fn analyze(function: &Function) -> Result<Report, String> {
    crate::mir::verify::verify(function)?;
    let cfg = Cfg::build(function)?;
    let mut origins = BTreeMap::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let InstructionKind::BorrowStart { id, kind, place } = &instruction.kind
                && origins.insert(*id, (*kind, place.clone())).is_some()
            {
                return Err(format!("MIR borrow #{} starts more than once", id.0));
            }
        }
    }
    let initial = State {
        locals: function
            .locals
            .iter()
            .map(|local| {
                if local.parameter_index.is_some() {
                    AVAILABLE
                } else {
                    UNINITIALIZED
                }
            })
            .collect(),
        borrows: BTreeSet::new(),
    };
    let states = solve_forward(
        function,
        &cfg,
        initial,
        |a, b| State {
            locals: a.locals.iter().zip(&b.locals).map(|(x, y)| x | y).collect(),
            borrows: a.borrows.union(&b.borrows).copied().collect(),
        },
        |block, incoming| {
            let mut state = incoming.clone();
            for instruction in &block.instructions {
                apply(&mut state, &instruction.kind, function);
            }
            state
        },
    );
    for block in &function.blocks {
        let Some(mut state) = states.entry[block.id.0].clone() else {
            continue;
        };
        for instruction in &block.instructions {
            check_instruction(function, &state, &instruction.kind, &origins)
                .map_err(|error| format!("MIR bb{}: {error}", block.id.0))?;
            apply(&mut state, &instruction.kind, function);
        }
        if matches!(
            &block.terminator.as_ref().expect("verified").kind,
            TerminatorKind::Return(_) | TerminatorKind::ReturnVoid
        ) && !state.borrows.is_empty()
        {
            return Err(format!(
                "MIR bb{} returns with an active borrow",
                block.id.0
            ));
        }
    }
    Ok(Report { cfg, states })
}

fn apply(state: &mut State, instruction: &InstructionKind, function: &Function) {
    match instruction {
        InstructionKind::Store { place, .. } | InstructionKind::Replace { place, .. }
            if place.projections.is_empty() =>
        {
            state.locals[place.root.0] = AVAILABLE
        }
        InstructionKind::Move(place)
            if function.locals[place.root.0].properties.copy == KnownProperty::No =>
        {
            state.locals[place.root.0] = MOVED
        }
        InstructionKind::Drop(place) => state.locals[place.root.0] = DROPPED,
        InstructionKind::BorrowStart { id, .. } => {
            state.borrows.insert(*id);
        }
        InstructionKind::EndBorrow(id) => {
            state.borrows.remove(id);
        }
        _ => {}
    }
}

fn check_instruction(
    function: &Function,
    state: &State,
    instruction: &InstructionKind,
    origins: &BTreeMap<BorrowId, (BorrowKind, Place)>,
) -> Result<(), String> {
    match instruction {
        InstructionKind::Load(place)
        | InstructionKind::CloneString(place)
        | InstructionKind::CloneManaged(place)
        | InstructionKind::ArrayLen(place)
        | InstructionKind::SnapshotArray(place) => {
            require_available(state, place)?;
            check_active_borrows(function, state, origins, place, false)?;
        }
        InstructionKind::Move(place) => {
            require_available(state, place)?;
            let consuming = function.locals[place.root.0].properties.copy == KnownProperty::No;
            check_active_borrows(function, state, origins, place, consuming)?;
        }
        InstructionKind::Drop(place) => {
            require_available(state, place)?;
            check_active_borrows(function, state, origins, place, true)?;
        }
        InstructionKind::Store { place, .. } => {
            check_active_borrows(function, state, origins, place, true)?;
            if place.projections.is_empty()
                && state.locals[place.root.0] == AVAILABLE
                && function.locals[place.root.0].properties.requires_drop == KnownProperty::Yes
            {
                return Err(format!(
                    "replacement of local #{} requires Drop after RHS evaluation",
                    place.root.0
                ));
            }
            if !place.projections.is_empty() {
                require_available(state, place)?;
            }
        }
        InstructionKind::Replace { place, .. } => {
            require_available(state, place)?;
            check_active_borrows(function, state, origins, place, true)?;
        }
        InstructionKind::BorrowStart { id, kind, place } => {
            require_available(state, place)?;
            if state.borrows.contains(id) {
                return Err(format!("borrow #{} is already active", id.0));
            }
            check_active_borrows(
                function,
                state,
                origins,
                place,
                *kind == BorrowKind::Mutable,
            )?;
        }
        InstructionKind::EndBorrow(id) => {
            if !state.borrows.contains(id) || !origins.contains_key(id) {
                return Err(format!("borrow #{} is not active", id.0));
            }
        }
        InstructionKind::Call { arguments, .. } => {
            let mut call_borrows = Vec::new();
            for argument in arguments {
                if let CallArgument::Place { mode, place } = argument {
                    require_available(state, place)?;
                    let mutable = *mode == ArgumentMode::BorrowMutable;
                    check_active_borrows(function, state, origins, place, mutable)?;
                    for (earlier, earlier_mutable) in &call_borrows {
                        if (mutable || *earlier_mutable) && may_overlap(function, place, earlier) {
                            return Err("conflicting call-scoped borrows".into());
                        }
                    }
                    call_borrows.push((place.clone(), mutable));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn require_available(state: &State, place: &Place) -> Result<(), String> {
    if state.locals[place.root.0] != AVAILABLE {
        return Err(format!(
            "local #{} is uninitialized, moved, dropped, or unavailable on some path",
            place.root.0
        ));
    }
    Ok(())
}

fn check_active_borrows(
    function: &Function,
    state: &State,
    origins: &BTreeMap<BorrowId, (BorrowKind, Place)>,
    place: &Place,
    mutable: bool,
) -> Result<(), String> {
    for id in &state.borrows {
        let (kind, earlier) = origins.get(id).ok_or("active borrow has no origin")?;
        if (mutable || *kind == BorrowKind::Mutable) && may_overlap(function, place, earlier) {
            return Err(format!("Place conflicts with active borrow #{}", id.0));
        }
    }
    Ok(())
}

/// Conservative overlap: unknown indices conflict. Distinct resolved fields
/// and distinct constant indices are disjoint.
pub fn may_overlap(function: &Function, left: &Place, right: &Place) -> bool {
    if left.root != right.root {
        return false;
    }
    for (a, b) in left.projections.iter().zip(&right.projections) {
        match (a, b) {
            (Projection::Field(a), Projection::Field(b)) if a != b => return false,
            (Projection::TupleField(a), Projection::TupleField(b)) if a != b => return false,
            (Projection::Index(a), Projection::Index(b)) => {
                if let (Some(x), Some(y)) =
                    (constant_index(function, *a), constant_index(function, *b))
                    && x != y
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn constant_index(function: &Function, id: TempId) -> Option<i64> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| {
            if instruction.result == Some(id) {
                if let InstructionKind::Const(Constant::Integer(value)) = instruction.kind {
                    Some(value)
                } else {
                    None
                }
            } else {
                None
            }
        })
}

pub fn local_state(
    report: &Report,
    block: crate::mir::ir::BlockId,
    local: LocalId,
) -> Option<&'static str> {
    let bits = report
        .states
        .entry
        .get(block.0)?
        .as_ref()?
        .locals
        .get(local.0)?;
    Some(match *bits {
        UNINITIALIZED => "Uninitialized",
        AVAILABLE => "Available",
        MOVED => "Moved",
        DROPPED => "Dropped",
        _ => "MaybeUnavailable",
    })
}
