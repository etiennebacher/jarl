//! Extension traits for AST nodes providing ergonomic helper methods.

use air_r_syntax::*;
use biome_rowan::AstNode;

/// Extension trait for R AST nodes providing common parent and sibling checks.
pub trait AstNodeExt: AstNode<Language = RLanguage> {
    /// Returns true if this node is the condition of an if statement.
    /// The condition is always at index 2: IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    fn parent_is_if_condition(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_IF_STATEMENT && self.syntax().index() == 2)
            .unwrap_or(false)
    }

    /// Returns true if this node is the body of an if statement.
    /// The body is always at index 4: IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    fn parent_is_if_body(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_IF_STATEMENT && self.syntax().index() == 4)
            .unwrap_or(false)
    }

    /// Returns true if this node is the condition of a while statement.
    /// The condition is always at index 2: WHILE_KW - L_PAREN - [condition] - R_PAREN - [body]
    fn parent_is_while_condition(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_WHILE_STATEMENT && self.syntax().index() == 2)
            .unwrap_or(false)
    }

    /// Returns true if this node is the body of a while statement.
    /// The body is always at index 4: WHILE_KW - L_PAREN - [condition] - R_PAREN - [body]
    fn parent_is_while_body(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_WHILE_STATEMENT && self.syntax().index() == 4)
            .unwrap_or(false)
    }

    /// Returns true if this node is the body of an else clause.
    /// The body is always at index 1: ELSE_KW - [alternative]
    fn parent_is_else_body(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_ELSE_CLAUSE && self.syntax().index() == 1)
            .unwrap_or(false)
    }

    /// Returns true if this node is the body of a for statement.
    /// The body is always at index 6: FOR_KW - L_PAREN - [variable] - IN_KW - [sequence] - R_PAREN - [body]
    fn parent_is_for_body(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_FOR_STATEMENT && self.syntax().index() == 6)
            .unwrap_or(false)
    }

    /// Returns true if this node has a pipe operator immediately before it.
    fn has_previous_pipe(&self) -> bool {
        self.syntax()
            .prev_sibling_or_token()
            .map(|prev| prev.kind() == RSyntaxKind::PIPE)
            .unwrap_or(false)
    }

    /// Returns true if parent is a unary expression with a BANG operator.
    /// This returns false for rlang's `!!` and `!!!`.
    fn parent_is_bang_unary(&self) -> bool {
        if let Some(parent) = self.syntax().parent()
            && parent.kind() == RSyntaxKind::R_UNARY_EXPRESSION
            && let Some(prev) = self.syntax().prev_sibling_or_token()
            && prev.kind() == RSyntaxKind::BANG
        {
            // Check if parent's parent is also a unary bang (double negation)
            if let Some(grandparent) = parent.parent()
                && grandparent.kind() == RSyntaxKind::R_UNARY_EXPRESSION
            {
                return false;
            }
            return true;
        }
        false
    }
}

// Blanket implementation for all R AST node types
impl<T> AstNodeExt for T where T: AstNode<Language = RLanguage> {}
