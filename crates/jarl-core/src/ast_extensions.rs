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

    /// Returns true if this node is the condition of a while statement.
    /// The condition is always at index 2: WHILE_KW - L_PAREN - [condition] - R_PAREN - [body]
    fn parent_is_while_condition(&self) -> bool {
        self.syntax()
            .parent()
            .map(|p| p.kind() == RSyntaxKind::R_WHILE_STATEMENT && self.syntax().index() == 2)
            .unwrap_or(false)
    }

    /// Returns true if this node has a ! (BANG) operator immediately before it.
    fn has_previous_bang(&self) -> bool {
        self.syntax()
            .prev_sibling_or_token()
            .map(|prev| prev.kind() == RSyntaxKind::BANG)
            .unwrap_or(false)
    }
}

// Blanket implementation for all R AST node types
impl<T> AstNodeExt for T where T: AstNode<Language = RLanguage> {}
