use super::graph::{BlockId, ControlFlowGraph, Terminator};
use air_r_syntax::{
    RBinaryExpression, RBracedExpressions, RForStatement, RFunctionDefinition, RIfStatement,
    RParenthesizedExpression, RRepeatStatement, RSyntaxKind, RSyntaxNode, RWhileStatement,
};
use biome_rowan::AstNode;

/// Builder for constructing control flow graphs
pub struct CfgBuilder {
    cfg: ControlFlowGraph,
    /// Stack of loop contexts for handling break/next
    loop_stack: Vec<LoopContext>,
}

/// Context information for a loop (for break/next targeting)
struct LoopContext {
    /// Block to jump to for 'next' (loop header/condition)
    continue_target: BlockId,
    /// Block to jump to for 'break' (after loop)
    break_target: BlockId,
}

/// Evaluate a constant boolean condition if possible
///
/// This handles:
/// - Direct TRUE/FALSE literals
/// - Binary expressions with `|`, `||`, `&`, `&&` where short-circuit logic applies:
///   - `TRUE | x` or `x | TRUE` → TRUE (regardless of x)
///   - `FALSE & x` or `x & FALSE` → FALSE (regardless of x)
///   - Same for `||` and `&&`
fn evaluate_constant_condition(node: &RSyntaxNode) -> Option<bool> {
    let kind = node.kind();

    // Handle direct TRUE/FALSE literals
    let is_true_literal = matches!(kind, RSyntaxKind::TRUE_KW | RSyntaxKind::R_TRUE_EXPRESSION);
    let is_false_literal = matches!(
        kind,
        RSyntaxKind::FALSE_KW | RSyntaxKind::R_FALSE_EXPRESSION
    );

    if is_true_literal {
        return Some(true);
    }
    if is_false_literal {
        return Some(false);
    }

    // Handle parenthesized expressions by unwrapping them
    if kind == RSyntaxKind::R_PARENTHESIZED_EXPRESSION
        && let Some(paren_expr) = RParenthesizedExpression::cast_ref(node)
        && let Ok(body) = paren_expr.body()
    {
        return evaluate_constant_condition(body.syntax());
    }

    // Handle binary expressions with boolean operators
    if kind == RSyntaxKind::R_BINARY_EXPRESSION
        && let Some(binary_expr) = RBinaryExpression::cast_ref(node)
        && let Ok(operator) = binary_expr.operator()
    {
        let op_kind = operator.kind();

        // Handle OR operators (| and ||)
        // TRUE | x → TRUE, x | TRUE → TRUE
        if matches!(op_kind, RSyntaxKind::OR | RSyntaxKind::OR2) {
            let left_val = binary_expr
                .left()
                .ok()
                .and_then(|e| evaluate_constant_condition(e.syntax()));
            let right_val = binary_expr
                .right()
                .ok()
                .and_then(|e| evaluate_constant_condition(e.syntax()));

            // If either side is TRUE, the whole expression is TRUE
            if left_val == Some(true) || right_val == Some(true) {
                return Some(true);
            }
            // If both sides are FALSE, the whole expression is FALSE
            if left_val == Some(false) && right_val == Some(false) {
                return Some(false);
            }
        }

        // Handle AND operators (& and &&)
        // FALSE & x → FALSE, x & FALSE → FALSE
        if matches!(op_kind, RSyntaxKind::AND | RSyntaxKind::AND2) {
            let left_val = binary_expr
                .left()
                .ok()
                .and_then(|e| evaluate_constant_condition(e.syntax()));
            let right_val = binary_expr
                .right()
                .ok()
                .and_then(|e| evaluate_constant_condition(e.syntax()));

            // If either side is FALSE, the whole expression is FALSE
            if left_val == Some(false) || right_val == Some(false) {
                return Some(false);
            }
            // If both sides are TRUE, the whole expression is TRUE
            if left_val == Some(true) && right_val == Some(true) {
                return Some(true);
            }
        }
    }

    None
}

impl CfgBuilder {
    fn new() -> Self {
        Self {
            cfg: ControlFlowGraph::new(),
            loop_stack: Vec::new(),
        }
    }

    /// Check if a block has any incoming edges (actual edges, not just predecessor pointers)
    ///
    /// This is used to detect when we're in an unreachable block (e.g., after an if/else
    /// where both branches terminate). In such cases, we don't want to recursively build
    /// control flow structures - we just want to add all statements to the unreachable block.
    fn has_incoming_edges(&self, block_id: BlockId) -> bool {
        // Entry block is always considered "reachable" for building purposes
        if block_id == self.cfg.entry {
            return true;
        }

        if let Some(block) = self.cfg.block(block_id) {
            // Check if any predecessor has an actual edge to this block
            for &pred_id in &block.predecessors {
                if let Some(pred) = self.cfg.block(pred_id)
                    && pred.successors.contains(&block_id)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Build a CFG from a function definition
    pub fn build(mut self, func: &RFunctionDefinition) -> ControlFlowGraph {
        let fields = func.as_fields();
        if let Ok(body) = fields.body {
            let entry = self.cfg.entry;
            let exit = self.cfg.exit;
            self.build_expression(body.syntax(), entry, exit);
        }

        self.cfg
    }

    /// Build CFG for any expression (handles AnyRExpression from loops)
    fn build_expression(&mut self, expr: &RSyntaxNode, current: BlockId, exit: BlockId) -> BlockId {
        // Check if it's a braced expression
        if let Some(braced) = RBracedExpressions::cast_ref(expr) {
            self.build_braced_expressions(&braced, current, exit)
        } else {
            // For non-braced expressions, treat as a single statement
            self.build_statement(expr, current, exit)
        }
    }

    /// Build CFG for a braced expression block
    fn build_braced_expressions(
        &mut self,
        braced: &RBracedExpressions,
        current: BlockId,
        exit: BlockId,
    ) -> BlockId {
        let fields = braced.as_fields();
        let expressions = fields.expressions;
        let items: Vec<_> = expressions
            .into_iter()
            .map(|e| e.syntax().clone())
            .collect();
        self.build_statements(&items, current, exit)
    }

    /// Build CFG for a sequence of statements
    fn build_statements(
        &mut self,
        statements: &[RSyntaxNode],
        mut current: BlockId,
        exit: BlockId,
    ) -> BlockId {
        for (idx, stmt) in statements.iter().enumerate() {
            current = self.build_statement(stmt, current, exit);

            // If we've hit a return or other terminator, remaining statements are unreachable
            if let Some(block) = self.cfg.block(current)
                && matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                )
            {
                // Create a new unreachable block for remaining statements
                if idx + 1 < statements.len() {
                    let unreachable = self.cfg.new_block();
                    // Mark this block as having the terminator block as predecessor (for tracking)
                    // Even though we don't add an edge, we store it in predecessors for analysis
                    if let Some(unreachable_block) = self.cfg.block_mut(unreachable) {
                        unreachable_block.predecessors.push(current);
                    }
                    // Add all remaining statements to the unreachable block
                    for remaining_stmt in &statements[idx + 1..] {
                        self.add_statement(unreachable, remaining_stmt.clone());
                    }
                    return unreachable;
                }
            }
        }
        current
    }

    /// Build CFG for a single statement
    fn build_statement(&mut self, stmt: &RSyntaxNode, current: BlockId, exit: BlockId) -> BlockId {
        // If current block has no incoming edges, we're in unreachable code.
        // Don't recursively build control flow structures - just add statements
        // to keep them grouped as a single diagnostic.
        if !self.has_incoming_edges(current) {
            self.add_statement(current, stmt.clone());
            return current;
        }

        match stmt.kind() {
            RSyntaxKind::R_BREAK_EXPRESSION => {
                self.build_break(current, stmt.clone());
                current
            }
            RSyntaxKind::R_NEXT_EXPRESSION => {
                self.build_next(current, stmt.clone());
                current
            }
            RSyntaxKind::R_RETURN_EXPRESSION => {
                self.build_return(current, stmt.clone());
                current
            }
            RSyntaxKind::R_IF_STATEMENT => {
                if let Some(if_stmt) = RIfStatement::cast_ref(stmt) {
                    self.build_if_statement(&if_stmt, current, exit)
                } else {
                    self.add_statement(current, stmt.clone());
                    current
                }
            }
            RSyntaxKind::R_FOR_STATEMENT => {
                if let Some(for_stmt) = RForStatement::cast_ref(stmt) {
                    self.build_for_statement(&for_stmt, current, exit)
                } else {
                    self.add_statement(current, stmt.clone());
                    current
                }
            }
            RSyntaxKind::R_WHILE_STATEMENT => {
                if let Some(while_stmt) = RWhileStatement::cast_ref(stmt) {
                    self.build_while_statement(&while_stmt, current, exit)
                } else {
                    self.add_statement(current, stmt.clone());
                    current
                }
            }
            RSyntaxKind::R_REPEAT_STATEMENT => {
                if let Some(repeat_stmt) = RRepeatStatement::cast_ref(stmt) {
                    self.build_repeat_statement(&repeat_stmt, current, exit)
                } else {
                    self.add_statement(current, stmt.clone());
                    current
                }
            }
            RSyntaxKind::R_CALL => {
                // Check if this is a return, break, or next call
                let fun_name = stmt.first_child().unwrap().text_trimmed().to_string();
                if fun_name == "return" {
                    self.build_return(current, stmt.clone());
                    current
                } else if fun_name == "break" {
                    self.build_break(current, stmt.clone());
                    current
                } else if fun_name == "next" {
                    self.build_next(current, stmt.clone());
                    current
                } else if ["stop", ".Defunct", "abort", "cli_abort", "q", "quit"]
                    .contains(&fun_name.as_str())
                {
                    self.build_stop(current, stmt.clone());
                    current
                } else {
                    self.add_statement(current, stmt.clone());
                    current
                }
            }
            RSyntaxKind::R_IDENTIFIER => {
                // Most identifiers are just regular statements
                self.add_statement(current, stmt.clone());
                current
            }
            _ => {
                self.add_statement(current, stmt.clone());
                current
            }
        }
    }

    /// Build CFG for if statement
    fn build_if_statement(
        &mut self,
        if_stmt: &RIfStatement,
        current: BlockId,
        exit: BlockId,
    ) -> BlockId {
        let fields = if_stmt.as_fields();
        let condition = fields.condition.ok().map(|c| c.syntax().clone());

        // Create blocks for then and else branches
        let then_block = self.cfg.new_block();
        let else_block = self.cfg.new_block();
        let after_if = self.cfg.new_block();

        // Check if the condition is a constant
        let constant_value = condition.as_ref().and_then(evaluate_constant_condition);

        // Set up the branch terminator
        if condition.is_some() {
            if let Some(block) = self.cfg.block_mut(current) {
                block.terminator = Terminator::Branch;
            }

            // Only add edges for branches that can actually be taken
            match constant_value {
                Some(true) => {
                    // Only then branch is reachable
                    self.cfg.add_edge(current, then_block);
                    // Mark else_block as unreachable by adding it as a predecessor
                    // but not connecting it from current
                    if let Some(else_b) = self.cfg.block_mut(else_block) {
                        else_b.predecessors.push(current);
                    }
                }
                Some(false) => {
                    // Only else branch is reachable
                    self.cfg.add_edge(current, else_block);
                    // Mark then_block as unreachable
                    if let Some(then_b) = self.cfg.block_mut(then_block) {
                        then_b.predecessors.push(current);
                    }
                }
                None => {
                    // Both branches are possible
                    self.cfg.add_edge(current, then_block);
                    self.cfg.add_edge(current, else_block);
                }
            }
        }

        // Build then branch
        if let Ok(consequence) = fields.consequence {
            // If this is a dead branch (condition is false), mark the entire branch as unreachable
            // by storing the whole branch syntax node
            if constant_value == Some(false) {
                // Dead then branch - store the entire branch as a single statement
                if let Some(block) = self.cfg.block_mut(then_block) {
                    block.statements.push(consequence.syntax().clone());
                }
            } else {
                // Reachable or maybe-reachable branch - build normally
                let then_end = self.build_expression(consequence.syntax(), then_block, exit);
                // Only add edge if the then block doesn't end with return/break/next
                // AND then_end is not itself an unreachable block (has incoming edges)
                if let Some(block) = self.cfg.block(then_end)
                    && !matches!(
                        block.terminator,
                        Terminator::Return
                            | Terminator::Break
                            | Terminator::Next
                            | Terminator::Stop
                    )
                    && self.has_incoming_edges(then_end)
                {
                    if let Some(b) = self.cfg.block_mut(then_end) {
                        b.terminator = Terminator::Goto;
                    }
                    self.cfg.add_edge(then_end, after_if);
                }
            }
        }

        // Build else branch if it exists
        if let Some(else_clause) = fields.else_clause {
            let else_fields = else_clause.as_fields();
            if let Ok(alt_body) = else_fields.alternative {
                // If this is a dead branch (condition is true), mark the entire branch as unreachable
                // by storing the whole branch syntax node
                if constant_value == Some(true) {
                    // Dead else branch - store the entire branch as a single statement
                    if let Some(block) = self.cfg.block_mut(else_block) {
                        block.statements.push(alt_body.syntax().clone());
                    }
                } else {
                    // Reachable or maybe-reachable branch - build normally
                    let else_end = self.build_expression(alt_body.syntax(), else_block, exit);
                    // Only add edge if the else block doesn't end with return/break/next
                    // AND else_end is not itself an unreachable block (has incoming edges)
                    if let Some(block) = self.cfg.block(else_end)
                        && !matches!(
                            block.terminator,
                            Terminator::Return
                                | Terminator::Break
                                | Terminator::Next
                                | Terminator::Stop
                        )
                        && self.has_incoming_edges(else_end)
                    {
                        if let Some(b) = self.cfg.block_mut(else_end) {
                            b.terminator = Terminator::Goto;
                        }
                        self.cfg.add_edge(else_end, after_if);
                    }
                }
            }
        } else {
            // No else branch, just connect to after_if
            if let Some(b) = self.cfg.block_mut(else_block) {
                b.terminator = Terminator::Goto;
            }
            self.cfg.add_edge(else_block, after_if);
        }

        // If after_if has no incoming edges (both branches terminated),
        // mark it as having the branch block as predecessor for proper
        // unreachable code reason detection
        if !self.has_incoming_edges(after_if)
            && let Some(after_block) = self.cfg.block_mut(after_if)
        {
            after_block.predecessors.push(current);
        }

        after_if
    }

    /// Build CFG for for loop
    fn build_for_statement(
        &mut self,
        for_stmt: &RForStatement,
        current: BlockId,
        exit: BlockId,
    ) -> BlockId {
        let fields = for_stmt.as_fields();
        let loop_header = self.cfg.new_block();
        let loop_body = self.cfg.new_block();
        let after_loop = self.cfg.new_block();

        // Jump from current to loop header
        if let Some(b) = self.cfg.block_mut(current) {
            b.terminator = Terminator::Goto;
        }
        self.cfg.add_edge(current, loop_header);

        // Loop header has the for condition/iterator
        if let Some(b) = self.cfg.block_mut(loop_header) {
            b.terminator = Terminator::Loop;
        }
        self.cfg.add_edge(loop_header, loop_body);
        self.cfg.add_edge(loop_header, after_loop);

        // Push loop context for break/next
        self.loop_stack.push(LoopContext {
            continue_target: loop_header,
            break_target: after_loop,
        });

        // Build loop body
        if let Ok(body) = fields.body {
            let body_end = self.build_expression(body.syntax(), loop_body, exit);
            // Loop back to header (unless body ends with return/break/next)
            if let Some(block) = self.cfg.block(body_end)
                && !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                )
            {
                if let Some(b) = self.cfg.block_mut(body_end) {
                    b.terminator = Terminator::Goto;
                }
                self.cfg.add_edge(body_end, loop_header);
            }
        }

        self.loop_stack.pop();

        after_loop
    }

    /// Build CFG for while loop
    fn build_while_statement(
        &mut self,
        while_stmt: &RWhileStatement,
        current: BlockId,
        exit: BlockId,
    ) -> BlockId {
        let fields = while_stmt.as_fields();
        let loop_header = self.cfg.new_block();
        let loop_body = self.cfg.new_block();
        let after_loop = self.cfg.new_block();

        // Jump from current to loop header
        if let Some(b) = self.cfg.block_mut(current) {
            b.terminator = Terminator::Goto;
        }
        self.cfg.add_edge(current, loop_header);

        // Loop header checks condition
        if let Some(b) = self.cfg.block_mut(loop_header) {
            b.terminator = Terminator::Loop;
        }
        self.cfg.add_edge(loop_header, loop_body);
        self.cfg.add_edge(loop_header, after_loop);

        // Push loop context
        self.loop_stack.push(LoopContext {
            continue_target: loop_header,
            break_target: after_loop,
        });

        // Build loop body
        if let Ok(body) = fields.body {
            let body_end = self.build_expression(body.syntax(), loop_body, exit);
            // Loop back to header
            if let Some(block) = self.cfg.block(body_end)
                && !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                )
            {
                if let Some(b) = self.cfg.block_mut(body_end) {
                    b.terminator = Terminator::Goto;
                }
                self.cfg.add_edge(body_end, loop_header);
            }
        }

        self.loop_stack.pop();

        after_loop
    }

    /// Build CFG for repeat loop
    fn build_repeat_statement(
        &mut self,
        repeat_stmt: &RRepeatStatement,
        current: BlockId,
        exit: BlockId,
    ) -> BlockId {
        let fields = repeat_stmt.as_fields();
        let loop_body = self.cfg.new_block();
        let after_loop = self.cfg.new_block();

        // Jump from current to loop body (repeat has no condition check)
        if let Some(b) = self.cfg.block_mut(current) {
            b.terminator = Terminator::Goto;
        }
        self.cfg.add_edge(current, loop_body);

        // Push loop context
        self.loop_stack.push(LoopContext {
            continue_target: loop_body,
            break_target: after_loop,
        });

        // Add edge to after_loop so it's always reachable (like for/while loops)
        // This represents the possibility of breaking out of the repeat loop
        self.cfg.add_edge(loop_body, after_loop);

        // Build loop body
        if let Ok(body) = fields.body {
            let body_end = self.build_expression(body.syntax(), loop_body, exit);
            // Loop back to start (infinite loop unless broken)
            if let Some(block) = self.cfg.block(body_end)
                && !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                )
            {
                if let Some(b) = self.cfg.block_mut(body_end) {
                    b.terminator = Terminator::Goto;
                }
                self.cfg.add_edge(body_end, loop_body);
            }
        }

        self.loop_stack.pop();

        after_loop
    }

    /// Build return statement
    fn build_return(&mut self, current: BlockId, _node: RSyntaxNode) {
        if let Some(block) = self.cfg.block_mut(current) {
            block.terminator = Terminator::Return;
        }
        // Return goes to exit (but we don't add edge since returns don't flow)
    }

    /// Build stop statement.
    ///
    /// This is a list of R functions that stop the execution, e.g. `stop()`,
    /// `abort()`, `cli_abort()`.
    fn build_stop(&mut self, current: BlockId, _node: RSyntaxNode) {
        if let Some(block) = self.cfg.block_mut(current) {
            block.terminator = Terminator::Stop;
        }
        // Return goes to exit (but we don't add edge since returns don't flow)
    }

    /// Build break statement
    fn build_break(&mut self, current: BlockId, _node: RSyntaxNode) {
        if let Some(loop_ctx) = self.loop_stack.last() {
            let target = loop_ctx.break_target;
            if let Some(block) = self.cfg.block_mut(current) {
                block.terminator = Terminator::Break;
            }
            self.cfg.add_edge(current, target);
        }
    }

    /// Build next statement
    fn build_next(&mut self, current: BlockId, _node: RSyntaxNode) {
        if let Some(loop_ctx) = self.loop_stack.last() {
            let target = loop_ctx.continue_target;
            if let Some(block) = self.cfg.block_mut(current) {
                block.terminator = Terminator::Next;
            }
            self.cfg.add_edge(current, target);
        }
    }

    /// Add a regular statement to a block
    fn add_statement(&mut self, block_id: BlockId, stmt: RSyntaxNode) {
        if let Some(block) = self.cfg.block_mut(block_id) {
            block.statements.push(stmt.clone());
            if let Some(current_range) = block.range {
                // Extend the range to include this statement
                let new_range = current_range.cover(stmt.text_trimmed_range());
                block.range = Some(new_range);
            } else {
                block.range = Some(stmt.text_trimmed_range());
            }
        }
    }
}

/// Build a control flow graph for a function definition
pub fn build_cfg(func: &RFunctionDefinition) -> ControlFlowGraph {
    let builder = CfgBuilder::new();
    builder.build(func)
}
