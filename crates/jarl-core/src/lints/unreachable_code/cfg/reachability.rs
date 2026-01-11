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
    /// Code after a statement to stop the execution (`stop()`, `abort()`, etc.)
    AfterStop,
    /// Code after a next statement
    AfterNext,
    /// Code in a branch that's never taken (constant condition)
    DeadBranch,
    /// Code that has no path from entry
    NoPathFromEntry,
}

/// Find all unreachable code in a control flow graph
///
/// This function:
/// 1. Uses BFS (Breadth-First Search) to find all reachable blocks from entry
/// 2. Identifies unreachable blocks and determines why they're unreachable
/// 3. Groups contiguous unreachable statements with the same reason into single diagnostics
pub fn find_unreachable_code(cfg: &ControlFlowGraph) -> Vec<UnreachableCodeInfo> {
    let mut unreachable = Vec::new();

    // Step 1: Find all blocks reachable from the entry block using BFS traversal
    let reachable = find_reachable_blocks(cfg);

    // Step 2: Collect all unreachable blocks with their info
    let mut unreachable_blocks: Vec<(TextRange, UnreachableReason)> = Vec::new();

    for block in &cfg.blocks {
        // Skip entry and exit blocks
        if block.id == cfg.entry || block.id == cfg.exit {
            continue;
        }

        if !reachable.contains(&block.id) {
            // This block is unreachable
            let reason = determine_unreachable_reason(cfg, block.id);

            // If block has statements, collect them
            if !block.statements.is_empty() {
                // Get the range spanning all statements in this block
                let first_stmt = &block.statements[0];
                let last_stmt = &block.statements[block.statements.len() - 1];
                let block_range = first_stmt
                    .text_trimmed_range()
                    .cover(last_stmt.text_trimmed_range());

                unreachable_blocks.push((block_range, reason));
            } else if let Some(range) = block.range {
                // Block has no statements but has a range
                unreachable_blocks.push((range, reason));
            }
        }
    }

    // Step 3: Sort by source position
    unreachable_blocks.sort_by_key(|(range, _)| range.start());

    // Step 4: Group contiguous unreachable code with the same reason
    // Since dead branches now collect all statements in a single block,
    // we only need to merge blocks that are directly contiguous (no gap)
    let mut current_group: Option<(TextRange, UnreachableReason)> = None;

    for (block_range, reason) in unreachable_blocks {
        if let Some((ref mut group_range, ref group_reason)) = current_group {
            let same_reason =
                std::mem::discriminant(group_reason) == std::mem::discriminant(&reason);
            let is_contiguous = block_range.start() == group_range.end();

            if same_reason && is_contiguous {
                // Extend the current group to cover this block
                *group_range = group_range.cover(block_range);
            } else {
                // Different reason or not contiguous - flush current group and start a new one
                unreachable.push(UnreachableCodeInfo {
                    range: *group_range,
                    reason: group_reason.clone(),
                });
                current_group = Some((block_range, reason));
            }
        } else {
            // Start a new group
            current_group = Some((block_range, reason));
        }
    }

    // Don't forget to flush any remaining group at the end
    if let Some((group_range, group_reason)) = current_group {
        unreachable.push(UnreachableCodeInfo { range: group_range, reason: group_reason });
    }

    unreachable
}

/// Determine why a block is unreachable by examining its context
///
/// Priority order:
/// 1. Check if a predecessor has a terminator (return/break/next)
/// 2. Check if it's a dead branch (has predecessor pointer but no actual edge)
/// 3. Check if any transitive predecessor is a dead branch
/// 4. Default to NoPathFromEntry
fn determine_unreachable_reason(cfg: &ControlFlowGraph, block_id: BlockId) -> UnreachableReason {
    use super::graph::Terminator;

    // Check the block's predecessors to find what terminator caused unreachability
    if let Some(block) = cfg.block(block_id) {
        // Priority 1: Check for terminators (return/break/next) - these take priority
        for &pred_id in &block.predecessors {
            if let Some(pred_block) = cfg.block(pred_id) {
                match &pred_block.terminator {
                    Terminator::Return => return UnreachableReason::AfterReturn,
                    Terminator::Break => return UnreachableReason::AfterBreak,
                    Terminator::Next => return UnreachableReason::AfterNext,
                    Terminator::Stop => return UnreachableReason::AfterStop,
                    _ => {}
                }
            }
        }

        // Priority 2: Check if it's a dead branch from a constant condition
        // Dead branches have a predecessor pointer (for tracking) but no actual CFG edge
        // This happens when we detect `if (TRUE)` or `if (FALSE)` during CFG construction
        if !block.predecessors.is_empty() {
            let has_incoming_edge = block.predecessors.iter().any(|&pred_id| {
                cfg.block(pred_id)
                    .map(|pred| pred.successors.contains(&block_id))
                    .unwrap_or(false)
            });

            if !has_incoming_edge {
                // Has predecessor but no actual edge - this is a dead branch from a constant condition
                return UnreachableReason::DeadBranch;
            }
        }
    }

    // Priority 3: Default reason
    UnreachableReason::NoPathFromEntry
}

/// Find all blocks reachable from the entry block using BFS (Breadth-First Search)
///
/// BFS is a graph traversal algorithm that explores nodes level by level:
/// - Start with the entry block
/// - Visit each reachable successor before moving to the next level
/// - Mark blocks as visited to avoid infinite loops
///
/// Time complexity: O(V + E) where V = number of blocks, E = number of edges
/// Space complexity: O(V) for the visited set and queue
fn find_reachable_blocks(cfg: &ControlFlowGraph) -> FxHashSet<BlockId> {
    let mut visited = FxHashSet::default();
    let mut queue = vec![cfg.entry];

    while let Some(block_id) = queue.pop() {
        if visited.insert(block_id) {
            // Add all successors to the queue (only following actual edges, not predecessor pointers)
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
