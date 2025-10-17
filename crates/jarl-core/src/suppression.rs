//! Comment-based suppression for lint rules
//!
//! This module handles extracting and checking `# jarl-skip` comments
//! to determine which nodes should skip linting.

use air_r_syntax::{RLanguage, RSyntaxNode};
use biome_formatter::comments::{CommentStyle, Comments};
use biome_rowan::SyntaxTriviaPieceComments;
use comments::{parse_comment_directive, Directive, LintDirective};
use std::collections::HashSet;

/// Comment style for R that identifies jarl-skip directives
#[derive(Default)]
pub struct RCommentStyle;

impl CommentStyle for RCommentStyle {
    type Language = RLanguage;

    fn is_suppression(_text: &str) -> bool {
        // We don't use biome's suppression tracking, so return false
        false
    }

    fn get_comment_kind(
        _comment: &SyntaxTriviaPieceComments<RLanguage>,
    ) -> biome_formatter::comments::CommentKind {
        // R only has line comments
        biome_formatter::comments::CommentKind::Line
    }

    fn place_comment(
        &self,
        comment: biome_formatter::comments::DecoratedComment<Self::Language>,
    ) -> biome_formatter::comments::CommentPlacement<Self::Language> {
        // Use default placement
        biome_formatter::comments::CommentPlacement::Default(comment)
    }
}

/// Tracks which nodes should skip linting based on comments
#[derive(Debug)]
pub struct SuppressionManager {
    comments: Comments<RLanguage>,
}

impl SuppressionManager {
    /// Create a new suppression manager from the root syntax node
    pub fn from_node(root: &RSyntaxNode) -> Self {
        let comments = Comments::from_node(root, &RCommentStyle, None);
        Self { comments }
    }

    /// Check if a node should skip all lints or specific rules
    ///
    /// Returns:
    /// - `Some(None)` if all lints should be skipped
    /// - `Some(Some(rules))` if specific rules should be skipped
    /// - `None` if linting should proceed normally
    pub fn check_suppression(&self, node: &RSyntaxNode) -> Option<Option<HashSet<String>>> {
        let leading = self.comments.leading_comments(node);

        // Check each leading comment for jarl-skip directives
        for comment in leading {
            let text = comment.piece().text();

            if let Some(Directive::Lint(directive)) = parse_comment_directive(text) {
                return match directive {
                    LintDirective::Skip => Some(None), // Skip all
                    LintDirective::SkipRules(rules) => Some(Some(rules.into_iter().collect())),
                };
            }
        }

        None
    }

    /// Check if a specific rule should be skipped for this node
    pub fn should_skip_rule(&self, node: &RSyntaxNode, rule_name: &str) -> bool {
        match self.check_suppression(node) {
            Some(None) => true, // Skip all
            Some(Some(rules)) => rules.contains(rule_name),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_r_parser::{parse, RParserOptions};
    use biome_rowan::AstNode;

    #[test]
    fn test_skip_all() {
        let code = r#"
# jarl-skip:
any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        assert_eq!(manager.check_suppression(first_expr), Some(None));
        assert!(manager.should_skip_rule(first_expr, "any_is_na"));
        assert!(manager.should_skip_rule(first_expr, "coalesce"));
    }

    #[test]
    fn test_skip_specific_rules() {
        let code = r#"
# jarl-skip: any_is_na, coalesce
any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        let suppressed = manager.check_suppression(first_expr);
        assert!(matches!(suppressed, Some(Some(_))));

        assert!(manager.should_skip_rule(first_expr, "any_is_na"));
        assert!(manager.should_skip_rule(first_expr, "coalesce"));
        assert!(!manager.should_skip_rule(first_expr, "scalar_in"));
    }

    #[test]
    fn test_no_suppression() {
        let code = r#"
# Just a regular comment
any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        assert_eq!(manager.check_suppression(first_expr), None);
        assert!(!manager.should_skip_rule(first_expr, "any_is_na"));
    }
}
