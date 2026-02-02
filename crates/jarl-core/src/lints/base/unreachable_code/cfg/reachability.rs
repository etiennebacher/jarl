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

#[derive(Debug, Clone, Copy)]
pub enum UnreachableReason {
    /// Code after a return statement
    AfterReturn,
    /// Code after a break statement
    AfterBreak,
    /// Code after a statement to stop the execution (`stop()`, `abort()`, etc.)
    AfterStop,
    /// Code after a next statement
    AfterNext,
    /// Code after an if/else where all branches terminate
    AfterBranchTerminating,
    /// Code in a branch that's never taken (constant condition)
    DeadBranch,
    /// Code that has no path from entry
    NoPathFromEntry,
}

impl UnreachableReason {
    /// Returns a human-readable message explaining why the code is unreachable
    pub fn message(&self) -> &'static str {
        match self {
            Self::AfterReturn => {
                "This code is unreachable because it appears after a return statement."
            }
            Self::AfterStop => {
                "This code is unreachable because it appears after a `stop()` statement (or equivalent)."
            }
            Self::AfterBreak => {
                "This code is unreachable because it appears after a break statement."
            }
            Self::AfterNext => {
                "This code is unreachable because it appears after a next statement."
            }
            Self::AfterBranchTerminating => {
                "This code is unreachable because the preceding if/else terminates in all branches."
            }
            Self::DeadBranch => "This code is in a branch that can never be executed.",
            Self::NoPathFromEntry => "This code has no execution path from the function entry.",
        }
    }
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
                unreachable
                    .push(UnreachableCodeInfo { range: *group_range, reason: *group_reason });
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
/// 2. Check if predecessor is a branch where all successors terminate (if/else with returns in all branches)
/// 3. Check if it's a dead branch (has predecessor pointer but no actual edge)
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

        // Priority 2: Check if predecessor is a branch where all successors terminate
        // This handles the case where an if/else has returns in all branches
        for &pred_id in &block.predecessors {
            if let Some(pred_block) = cfg.block(pred_id)
                && matches!(pred_block.terminator, Terminator::Branch)
            {
                // Check all successors of the branch to find what terminator they end with
                if let Some(reason) = find_branch_terminator_reason(cfg, pred_id) {
                    return reason;
                }
            }
        }

        // Priority 3: Check if it's a dead branch from a constant condition
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

    // Priority 4: Default reason
    UnreachableReason::NoPathFromEntry
}

/// Find the terminator reason for a branch by traversing its successors
///
/// This handles if/else statements where all branches terminate (return/break/next/stop).
/// We traverse the branch's successors to find what terminator they end with.
fn find_branch_terminator_reason(
    cfg: &ControlFlowGraph,
    branch_id: BlockId,
) -> Option<UnreachableReason> {
    let branch_block = cfg.block(branch_id)?;

    // We need ALL branches to terminate for code after the if/else to be unreachable
    if branch_block.successors.is_empty() {
        return None;
    }

    // Use a shared cache for all branches to avoid recomputing paths
    let mut cache = rustc_hash::FxHashMap::default();

    // Collect terminators from all branches
    let mut all_terminate = true;
    let mut found_return = false;
    let mut found_stop = false;

    for &succ_id in &branch_block.successors {
        if let Some(reason) = find_terminator_in_path(cfg, succ_id, &mut cache) {
            match reason {
                UnreachableReason::AfterReturn => found_return = true,
                UnreachableReason::AfterStop => found_stop = true,
                // Break/Next inside an if/else would exit to a loop, not make code after if unreachable
                _ => {}
            }
        } else {
            // This branch doesn't terminate - code after the if/else is reachable
            all_terminate = false;
        }
    }

    // Only return AfterBranchTerminating if ALL branches terminate
    if all_terminate && (found_return || found_stop) {
        Some(UnreachableReason::AfterBranchTerminating)
    } else {
        None
    }
}

/// Recursively find what terminator a path ends with
///
/// Uses a cache to store computed results, avoiding both infinite loops and redundant computation.
/// When multiple branches converge to the same terminator, we return the cached result.
fn find_terminator_in_path(
    cfg: &ControlFlowGraph,
    block_id: BlockId,
    cache: &mut rustc_hash::FxHashMap<BlockId, Option<UnreachableReason>>,
) -> Option<UnreachableReason> {
    use super::graph::Terminator;

    // Check cache first
    if let Some(&cached) = cache.get(&block_id) {
        return cached;
    }

    // Mark as in-progress (None) to detect cycles
    cache.insert(block_id, None);

    let block = cfg.block(block_id)?;

    let result = match &block.terminator {
        Terminator::Return => Some(UnreachableReason::AfterReturn),
        Terminator::Stop => Some(UnreachableReason::AfterStop),
        Terminator::Break | Terminator::Next => {
            // These exit to loop structures, not relevant for if/else termination
            None
        }
        Terminator::Goto => {
            // Follow the goto to its successor
            if let Some(&next) = block.successors.first() {
                find_terminator_in_path(cfg, next, cache)
            } else {
                None
            }
        }
        Terminator::Branch => {
            // Nested if - check if all branches terminate
            let mut result = None;
            for &succ_id in &block.successors {
                if let Some(reason) = find_terminator_in_path(cfg, succ_id, cache) {
                    result = Some(reason);
                } else {
                    // One branch doesn't terminate, so the nested if doesn't fully terminate
                    cache.insert(block_id, None);
                    return None;
                }
            }
            result
        }
        Terminator::Loop | Terminator::None => None,
    };

    // Cache the result
    cache.insert(block_id, result);
    result
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
