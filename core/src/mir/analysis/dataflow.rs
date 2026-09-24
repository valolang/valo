use std::collections::BTreeSet;

use super::cfg::Cfg;
use crate::mir::ir::{BasicBlock, Function};

/// `None` denotes an unreachable block. Joins and transfers must be monotone
/// over a finite lattice for cyclic CFGs to converge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardResult<S> {
    pub entry: Vec<Option<S>>,
    pub exit: Vec<Option<S>>,
}

pub fn solve_forward<S: Clone + Eq>(
    function: &Function,
    cfg: &Cfg,
    entry_state: S,
    join: impl Fn(&S, &S) -> S,
    transfer: impl Fn(&BasicBlock, &S) -> S,
) -> ForwardResult<S> {
    let mut entry = vec![None; function.blocks.len()];
    let mut exit = vec![None; function.blocks.len()];
    entry[function.entry.0] = Some(entry_state);
    let mut pending = BTreeSet::from([function.entry.0]);
    while let Some(index) = pending.pop_first() {
        let input = entry[index].as_ref().expect("queued block has an input");
        let output = transfer(&function.blocks[index], input);
        if exit[index].as_ref() == Some(&output) {
            continue;
        }
        exit[index] = Some(output.clone());
        for successor in &cfg.successors[index] {
            let slot = &mut entry[successor.0];
            let merged = slot
                .as_ref()
                .map_or_else(|| output.clone(), |old| join(old, &output));
            if slot.as_ref() != Some(&merged) {
                *slot = Some(merged);
                pending.insert(successor.0);
            }
        }
    }
    ForwardResult { entry, exit }
}
