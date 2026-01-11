use super::graph::{BlockId, ControlFlowGraph, Terminator};
use air_r_syntax::{
    RBracedExpressions, RForStatement, RFunctionDefinition, RIfStatement, RRepeatStatement,
    RSyntaxKind, RSyntaxNode, RWhileStatement,
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
fn evaluate_constant_condition(node: &RSyntaxNode) -> Option<bool> {
    let text = node.text_trimmed().to_string();
    let trimmed = text.trim();

    match trimmed {
        "TRUE" => Some(true),
        "FALSE" => Some(false),
        _ => None,
    }
}

impl CfgBuilder {
    fn new() -> Self {
        Self {
            cfg: ControlFlowGraph::new(),
            loop_stack: Vec::new(),
        }
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
            if let Some(block) = self.cfg.block(current) {
                if matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                ) {
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
        }
        current
    }

    /// Build CFG for a single statement
    fn build_statement(&mut self, stmt: &RSyntaxNode, current: BlockId, exit: BlockId) -> BlockId {
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
                let fun_name = stmt.first_child().unwrap().to_string();
                let fun_name = fun_name.trim();
                if fun_name.trim().starts_with("return") {
                    self.build_return(current, stmt.clone());
                    current
                } else if fun_name == "break" {
                    self.build_break(current, stmt.clone());
                    current
                } else if fun_name == "next" {
                    self.build_next(current, stmt.clone());
                    current
                } else if ["stop", "abort", "cli_abort"].contains(&fun_name) {
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
        let constant_value = condition
            .as_ref()
            .and_then(|c| evaluate_constant_condition(c));

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
            let then_end = self.build_expression(consequence.syntax(), then_block, exit);
            // Only add edge if the then block doesn't end with return/break/next
            if let Some(block) = self.cfg.block(then_end) {
                if !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                ) {
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
                let else_end = self.build_expression(alt_body.syntax(), else_block, exit);
                // Only add edge if the else block doesn't end with return/break/next
                if let Some(block) = self.cfg.block(else_end) {
                    if !matches!(
                        block.terminator,
                        Terminator::Return { .. }
                            | Terminator::Break { .. }
                            | Terminator::Next { .. }
                            | Terminator::Stop { .. }
                    ) {
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
            if let Some(block) = self.cfg.block(body_end) {
                if !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                ) {
                    if let Some(b) = self.cfg.block_mut(body_end) {
                        b.terminator = Terminator::Goto;
                    }
                    self.cfg.add_edge(body_end, loop_header);
                }
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
            if let Some(block) = self.cfg.block(body_end) {
                if !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                ) {
                    if let Some(b) = self.cfg.block_mut(body_end) {
                        b.terminator = Terminator::Goto;
                    }
                    self.cfg.add_edge(body_end, loop_header);
                }
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

        // Build loop body
        if let Ok(body) = fields.body {
            let body_end = self.build_expression(body.syntax(), loop_body, exit);
            // Loop back to start (infinite loop unless broken)
            if let Some(block) = self.cfg.block(body_end) {
                if !matches!(
                    block.terminator,
                    Terminator::Return | Terminator::Break | Terminator::Next | Terminator::Stop
                ) {
                    if let Some(b) = self.cfg.block_mut(body_end) {
                        b.terminator = Terminator::Goto;
                    }
                    self.cfg.add_edge(body_end, loop_body);
                }
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
            if block.range.is_none() {
                block.range = Some(stmt.text_trimmed_range());
            } else {
                // Extend the range to include this statement
                let current_range = block.range.unwrap();
                let new_range = current_range.cover(stmt.text_trimmed_range());
                block.range = Some(new_range);
            }
        }
    }
}

/// Build a control flow graph for a function definition
pub fn build_cfg(func: &RFunctionDefinition) -> ControlFlowGraph {
    let builder = CfgBuilder::new();
    builder.build(func)
}
