use std::collections::VecDeque;

use crate::mir::ir::{BlockId, Function, TerminatorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub successors: Vec<Vec<BlockId>>,
    pub predecessors: Vec<Vec<BlockId>>,
    pub reachable: Vec<bool>,
    pub reverse_postorder: Vec<BlockId>,
}

impl Cfg {
    pub fn build(function: &Function) -> Result<Self, String> {
        let count = function.blocks.len();
        if function.entry.0 >= count {
            return Err("CFG entry block is invalid".into());
        }
        let mut successors = Vec::with_capacity(count);
        let mut predecessors = vec![Vec::new(); count];
        for block in &function.blocks {
            let targets = match &block
                .terminator
                .as_ref()
                .ok_or("CFG block has no terminator")?
                .kind
            {
                TerminatorKind::Goto(target) => vec![*target],
                TerminatorKind::Branch {
                    then_block,
                    else_block,
                    ..
                } if then_block == else_block => vec![*then_block],
                TerminatorKind::Branch {
                    then_block,
                    else_block,
                    ..
                } => vec![*then_block, *else_block],
                TerminatorKind::Return(_)
                | TerminatorKind::Trap(_)
                | TerminatorKind::Unreachable => Vec::new(),
            };
            for target in &targets {
                if target.0 >= count {
                    return Err("CFG edge targets an invalid block".into());
                }
                predecessors[target.0].push(block.id);
            }
            successors.push(targets);
        }
        let mut reachable = vec![false; count];
        let mut queue = VecDeque::from([function.entry]);
        reachable[function.entry.0] = true;
        while let Some(block) = queue.pop_front() {
            for target in &successors[block.0] {
                if !reachable[target.0] {
                    reachable[target.0] = true;
                    queue.push_back(*target);
                }
            }
        }
        let mut seen = vec![false; count];
        let mut postorder = Vec::new();
        let mut stack = vec![(function.entry, 0usize)];
        seen[function.entry.0] = true;
        while let Some((block, next)) = stack.last_mut() {
            if *next == successors[block.0].len() {
                postorder.push(*block);
                stack.pop();
            } else {
                let target = successors[block.0][*next];
                *next += 1;
                if !seen[target.0] {
                    seen[target.0] = true;
                    stack.push((target, 0));
                }
            }
        }
        postorder.reverse();
        Ok(Self {
            successors,
            predecessors,
            reachable,
            reverse_postorder: postorder,
        })
    }
}
