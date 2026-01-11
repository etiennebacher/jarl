use super::graph::{BlockId, ControlFlowGraph};
use air_r_syntax::TextRange;
use rustc_hash::FxHashSet;

/// Information about unreachable code found in a CFG
#[derive(Debug, Clone)]
pub struct UnreachableCodeInfo {
    /// The text range of the unreachable code (for diagnostics)
    pub range: TextRange,
    /// Why this code is unreachable
    pub reason: UnreachableReason,
}

#[derive(Debug, Clone)]
pub enum UnreachableReason {
    /// Code after a return statement
    AfterReturn,
    /// Code after a break statement
    AfterBreak,
    /// Code after a next statement
    AfterNext,
    /// Code that has no path from entry
    NoPathFromEntry,
}

/// Find all unreachable code in a control flow graph
pub fn find_unreachable_code(cfg: &ControlFlowGraph) -> Vec<UnreachableCodeInfo> {
    let mut unreachable = Vec::new();

    // First, find blocks reachable from entry
    let reachable = find_reachable_blocks(cfg);

    // Any block not reachable from entry (except exit) is unreachable
    for block in &cfg.blocks {
        // Skip entry and exit blocks
        if block.id == cfg.entry || block.id == cfg.exit {
            continue;
        }

        if !reachable.contains(&block.id) {
            // Determine the reason by checking what makes this unreachable
            let reason = determine_unreachable_reason(cfg, block.id);

            // Report each statement in the unreachable block individually
            if !block.statements.is_empty() {
                for stmt in &block.statements {
                    unreachable.push(UnreachableCodeInfo {
                        range: stmt.text_trimmed_range(),
                        reason: reason.clone(),
                    });
                }
            } else if let Some(range) = block.range {
                // If block has no statements but has a range, report that
                unreachable.push(UnreachableCodeInfo {
                    range,
                    reason: reason.clone(),
                });
            }
        }
    }

    unreachable
}

/// Determine why a block is unreachable by examining its context
fn determine_unreachable_reason(cfg: &ControlFlowGraph, block_id: BlockId) -> UnreachableReason {
    use super::graph::Terminator;

    // Check the block's predecessors to find what terminator caused unreachability
    if let Some(block) = cfg.block(block_id) {
        for &pred_id in &block.predecessors {
            if let Some(pred_block) = cfg.block(pred_id) {
                match &pred_block.terminator {
                    Terminator::Return => return UnreachableReason::AfterReturn,
                    Terminator::Break => return UnreachableReason::AfterBreak,
                    Terminator::Next => return UnreachableReason::AfterNext,
                    _ => {}
                }
            }
        }
    }

    UnreachableReason::NoPathFromEntry
}

/// Find all blocks reachable from the entry block using BFS
fn find_reachable_blocks(cfg: &ControlFlowGraph) -> FxHashSet<BlockId> {
    let mut visited = FxHashSet::default();
    let mut queue = vec![cfg.entry];

    while let Some(block_id) = queue.pop() {
        if visited.insert(block_id) {
            // Add all successors to the queue
            if let Some(block) = cfg.block(block_id) {
                for &successor in &block.successors {
                    if !visited.contains(&successor) {
                        queue.push(successor);
                    }
                }
            }
        }
    }

    visited
}
