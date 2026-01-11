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

    // Process blocks in order and group contiguous unreachable ones
    let mut current_group: Option<(TextRange, UnreachableReason)> = None;

    for block in &cfg.blocks {
        // Skip entry and exit blocks
        if block.id == cfg.entry || block.id == cfg.exit {
            continue;
        }

        if !reachable.contains(&block.id) {
            // This block is unreachable
            let reason = determine_unreachable_reason(cfg, block.id);

            // If block has statements, process them
            if !block.statements.is_empty() {
                // Get the range spanning all statements in this block
                let first_stmt = &block.statements[0];
                let last_stmt = &block.statements[block.statements.len() - 1];
                let block_range = first_stmt
                    .text_trimmed_range()
                    .cover(last_stmt.text_trimmed_range());

                // Try to merge with current group if reasons match
                if let Some((ref mut group_range, ref group_reason)) = current_group {
                    if std::mem::discriminant(group_reason) == std::mem::discriminant(&reason) {
                        // Same reason, extend the range
                        *group_range = group_range.cover(block_range);
                    } else {
                        // Different reason, push the current group and start a new one
                        unreachable.push(UnreachableCodeInfo {
                            range: *group_range,
                            reason: group_reason.clone(),
                        });
                        current_group = Some((block_range, reason));
                    }
                } else {
                    // No current group, start a new one
                    current_group = Some((block_range, reason));
                }
            } else if let Some(range) = block.range {
                // Block has no statements but has a range
                if let Some((ref mut group_range, ref group_reason)) = current_group {
                    if std::mem::discriminant(group_reason) == std::mem::discriminant(&reason) {
                        *group_range = group_range.cover(range);
                    } else {
                        unreachable.push(UnreachableCodeInfo {
                            range: *group_range,
                            reason: group_reason.clone(),
                        });
                        current_group = Some((range, reason));
                    }
                } else {
                    current_group = Some((range, reason));
                }
            }
        } else {
            // Block is reachable, flush any current group
            if let Some((group_range, group_reason)) = current_group.take() {
                unreachable.push(UnreachableCodeInfo {
                    range: group_range,
                    reason: group_reason,
                });
            }
        }
    }

    // Don't forget to flush any remaining group at the end
    if let Some((group_range, group_reason)) = current_group {
        unreachable.push(UnreachableCodeInfo {
            range: group_range,
            reason: group_reason,
        });
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
