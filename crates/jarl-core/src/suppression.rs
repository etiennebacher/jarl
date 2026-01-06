//! Comment-based suppression for lint rules
//!
//! This module handles extracting and checking `# nolint` comments
//! to determine which nodes should skip linting.

use air_r_syntax::{RLanguage, RSyntaxKind, RSyntaxNode};
use biome_formatter::comments::{
    CommentKind, CommentPlacement, CommentStyle, Comments, DecoratedComment,
};
use biome_rowan::{SyntaxTriviaPieceComments, TextRange};
use std::collections::HashSet;

use crate::directive::{LintDirective, parse_comment_directive, parse_special_skip_file};
use crate::rule_set::Rule;
/// Comment style for R that identifies nolint directives
#[derive(Default)]
pub struct RCommentStyle;

impl CommentStyle for RCommentStyle {
    type Language = RLanguage;

    fn is_suppression(_text: &str) -> bool {
        // We don't use biome's suppression tracking, so return false
        false
    }

    fn get_comment_kind(_comment: &SyntaxTriviaPieceComments<RLanguage>) -> CommentKind {
        // R only has line comments
        CommentKind::Line
    }

    fn place_comment(
        &self,
        comment: DecoratedComment<Self::Language>,
    ) -> CommentPlacement<Self::Language> {
        // If the comment is attached to an R_CALL_ARGUMENTS node, find the
        // R_ARGUMENT_LIST and attach to its R_ARGUMENT's first child instead.
        // Without this, the comment below is attached to the R_ARGUMENT instead
        // of R_CALL, meaning that "# nolint" is useless:
        // ```
        // foo(
        //   # nolint
        //   any(is.na(x))
        // )
        // ```
        //
        // This is also more efficient than checking whether the parent is an
        // R_ARGUMENT in `should_skip_rule()`.
        let enclosing = comment.enclosing_node();
        if enclosing.kind() == RSyntaxKind::R_CALL_ARGUMENTS {
            // Find R_ARGUMENT_LIST child, then first R_ARGUMENT, then its first child
            for child in enclosing.children() {
                if child.kind() == RSyntaxKind::R_ARGUMENT_LIST
                    && let Some(first_arg) = child.first_child()
                    && first_arg.kind() == RSyntaxKind::R_ARGUMENT
                    && let Some(first_child) = first_arg.first_child()
                {
                    return CommentPlacement::leading(first_child, comment);
                }
            }
        }

        CommentPlacement::Default(comment)
    }
}

/// Represents a region where linting should be skipped
#[derive(Debug, Clone)]
struct SkipRegion {
    /// The range of text covered by this skip region
    range: TextRange,
    /// Rules to skip (None means skip all rules)
    rules: Option<HashSet<Rule>>,
}

/// Tracks which nodes should skip linting based on comments
#[derive(Debug)]
pub struct SuppressionManager {
    comments: Comments<RLanguage>,
    /// Regions defined by nolint start/end blocks
    skip_regions: Vec<SkipRegion>,
    /// Fast path: true if there are no suppressions anywhere in the file
    has_any_suppressions: bool,
}

impl SuppressionManager {
    /// Create a new suppression manager from the root syntax node
    pub fn from_node(root: &RSyntaxNode) -> Self {
        let comments = Comments::from_node(root, &RCommentStyle, None);
        let skip_regions = Self::build_skip_regions(root, &comments);

        // Check if there are any suppressions at all
        let has_any_suppressions =
            !skip_regions.is_empty() || Self::has_any_directives(root, &comments);

        Self { comments, skip_regions, has_any_suppressions }
    }

    /// Check if there are any nolint directives in comments
    fn has_any_directives(node: &RSyntaxNode, comments: &Comments<RLanguage>) -> bool {
        // Check all comment types for this node
        for comment in comments
            .leading_comments(node)
            .iter()
            .chain(comments.trailing_comments(node))
            .chain(comments.dangling_comments(node))
        {
            let text = comment.piece().text();
            if parse_comment_directive(text).is_some() {
                return true;
            }
        }

        // Recursively check children
        for child in node.children() {
            if Self::has_any_directives(&child, comments) {
                return true;
            }
        }

        false
    }

    /// Build skip regions from nolint start/end directives
    fn build_skip_regions(root: &RSyntaxNode, comments: &Comments<RLanguage>) -> Vec<SkipRegion> {
        let mut regions = Vec::new();
        let mut stack: Vec<(TextRange, Option<Vec<Rule>>)> = Vec::new();

        Self::collect_start_end_directives(root, comments, &mut stack, &mut regions);

        regions
    }

    fn collect_start_end_directives(
        node: &RSyntaxNode,
        comments: &Comments<RLanguage>,
        stack: &mut Vec<(TextRange, Option<Vec<Rule>>)>,
        regions: &mut Vec<SkipRegion>,
    ) {
        // Check all comment types for this node
        for comment in comments
            .leading_comments(node)
            .iter()
            .chain(comments.trailing_comments(node))
            .chain(comments.dangling_comments(node))
        {
            let text = comment.piece().text();
            let range = comment.piece().text_range();

            match parse_comment_directive(text) {
                Some(LintDirective::SkipStart) => {
                    // Start skipping all rules
                    stack.push((range, None));
                }
                Some(LintDirective::SkipStartRules(rule_names)) => {
                    // Start skipping specific rules - convert strings to Rule enums
                    let rules: Vec<Rule> = rule_names
                        .into_iter()
                        .filter_map(|name| Rule::from_name(&name))
                        .collect();
                    stack.push((range, Some(rules)));
                }
                Some(LintDirective::SkipEnd) => {
                    // End the most recent skip block
                    if let Some((start_range, rules)) = stack.pop() {
                        regions.push(SkipRegion {
                            range: TextRange::new(start_range.start(), range.end()),
                            rules: rules.map(|r| r.into_iter().collect()),
                        });
                    }
                }
                _ => {}
            }
        }

        // Recursively process children
        for child in node.children() {
            Self::collect_start_end_directives(&child, comments, stack, regions);
        }
    }

    /// Check if the entire file should be skipped
    ///
    /// This checks for special skip file comments like "# Generated by".
    /// Air also parses special comments "fmt: skip file", but Jarl doesn't
    /// provide an equivalent to this because we already list rule names after
    /// the colon.
    ///
    /// Returns `true` if the file should be skipped entirely.
    pub fn should_skip_file(&self, root: &RSyntaxNode) -> bool {
        self.check_skip_file_in_node(root)
    }

    fn check_skip_file_in_node(&self, node: &RSyntaxNode) -> bool {
        // A root node always has children, even if the file is empty.
        // We want to check the presence of a "Generated by" text only in the
        // first block of comments, meaning only in the leading comments of the
        // first child of the root node.
        //
        // Example:
        //
        // - those skip the file:
        //
        // ```
        // # generated by foo
        // 1 + 1
        // ```
        //
        // ```
        // # hello there
        // # generated by foo
        // 1 + 1
        // ```
        //
        // - this doesn't skip the file:
        //
        // ```
        // 1 + 1
        // # generated by foo
        // 2 + 2
        // ```
        let r_expression_list = node.first_child().unwrap();
        let first_child = r_expression_list.first_child();

        if let Some(child) = first_child {
            for comment in self.comments.leading_comments(&child) {
                let text = comment.piece().text();
                if let Some(LintDirective::SkipFile) = parse_special_skip_file(text) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a node should skip all lints or specific rules
    ///
    /// Returns:
    /// - `Some(None)` if all lints should be skipped
    /// - `Some(Some(rules))` if specific rules should be skipped
    /// - `None` if linting should proceed normally
    pub fn check_suppression(&self, node: &RSyntaxNode) -> Option<Option<HashSet<Rule>>> {
        // Check if node is in a skip region
        let node_range = node.text_trimmed_range();
        let mut region_suppression: Option<Option<HashSet<Rule>>> = None;
        for region in &self.skip_regions {
            if region.range.contains_range(node_range) {
                region_suppression = Some(region.rules.clone());
                break;
            }
        }

        // Check for node-level directives
        // Helper function to check comments for nolint directives
        let check_comments = |comments: &[biome_formatter::comments::SourceComment<
            RLanguage,
        >]|
         -> Option<Option<HashSet<Rule>>> {
            for comment in comments {
                let text = comment.piece().text();

                match parse_comment_directive(text) {
                    Some(directive) => {
                        return match directive {
                            LintDirective::Skip => Some(None), // Skip all
                            LintDirective::SkipRules(rule_names) => {
                                // Convert rule names to Rule enums
                                let rules: HashSet<Rule> = rule_names
                                    .into_iter()
                                    .filter_map(|name| Rule::from_name(&name))
                                    .collect();
                                Some(Some(rules))
                            }
                            LintDirective::SkipFile => {
                                // SkipFile directives are handled at file level via should_skip_file()
                                // If we encounter one here, treat it as skip all for this node
                                Some(None)
                            }
                            LintDirective::SkipStart
                            | LintDirective::SkipStartRules(_)
                            | LintDirective::SkipEnd => {
                                // Start/End directives are handled by skip regions
                                // Continue checking other comments for node-level directives
                                continue;
                            }
                        };
                    }
                    None => {
                        // Not a directive, continue checking other comments
                    }
                }
            }
            None
        };

        // Check leading comments
        let leading = self.comments.leading_comments(node);
        if let Some(result) = check_comments(leading) {
            return Some(result);
        }

        // Check trailing comments
        let trailing = self.comments.trailing_comments(node);
        if let Some(result) = check_comments(trailing) {
            return Some(result);
        }

        // Check dangling comments
        let dangling = self.comments.dangling_comments(node);
        if let Some(result) = check_comments(dangling) {
            return Some(result);
        }

        // If no node-level directive, return region suppression
        region_suppression
    }

    /// Check if a specific rule should be skipped for this node
    pub fn should_skip_rule(&self, node: &RSyntaxNode, rule: Rule) -> bool {
        // Fast path: if there are no suppressions anywhere, return immediately
        if !self.has_any_suppressions {
            return false;
        }
        // Check skip regions first
        let node_range = node.text_trimmed_range();
        for region in &self.skip_regions {
            if region.range.contains_range(node_range) {
                match &region.rules {
                    None => return true, // Skip all
                    Some(rules) => {
                        if rules.contains(&rule) {
                            return true;
                        }
                    }
                }
            }
        }

        // Check node-level suppression
        match self.check_suppression(node) {
            Some(None) => return true, // Skip all
            Some(Some(rules)) => {
                if rules.contains(&rule) {
                    return true;
                }
            }
            None => {}
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_r_parser::{RParserOptions, parse};
    use biome_rowan::AstNode;

    #[test]
    fn test_skip_all() {
        let code = r#"
# nolint
any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        assert_eq!(manager.check_suppression(first_expr), Some(None));
        assert!(manager.should_skip_rule(first_expr, Rule::AnyIsNa));
        assert!(manager.should_skip_rule(first_expr, Rule::Coalesce));
    }

    #[test]
    fn test_skip_specific_rules() {
        let code = r#"
# nolint: any_is_na, coalesce
any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        let suppressed = manager.check_suppression(first_expr);
        assert!(matches!(suppressed, Some(Some(_))));

        assert!(manager.should_skip_rule(first_expr, Rule::AnyIsNa));
        assert!(manager.should_skip_rule(first_expr, Rule::Coalesce));
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
        assert!(!manager.should_skip_rule(first_expr, Rule::AnyIsNa));
    }

    #[test]
    fn test_trailing_skip_all() {
        let code = r#"any(is.na(x)) # nolint"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        assert_eq!(manager.check_suppression(first_expr), Some(None));
        assert!(manager.should_skip_rule(first_expr, Rule::AnyIsNa));
        assert!(manager.should_skip_rule(first_expr, Rule::Coalesce));
    }

    #[test]
    fn test_trailing_skip_specific_rules() {
        let code = r#"any(is.na(x)) # nolint: any_is_na, coalesce"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        let expressions: Vec<_> = parsed.tree().expressions().into_iter().collect();
        let first_expr = expressions[0].syntax();

        let suppressed = manager.check_suppression(first_expr);
        assert!(matches!(suppressed, Some(Some(_))));

        assert!(manager.should_skip_rule(first_expr, Rule::AnyIsNa));
        assert!(manager.should_skip_rule(first_expr, Rule::Coalesce));
    }

    #[test]
    fn test_skip_file_generated_by() {
        let code = r#"
# Generated by roxygen2: do not edit by hand
# This is a roxygen-generated file

any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        // Should detect the file should be skipped
        assert!(manager.should_skip_file(&parsed.syntax()));
    }

    #[test]
    fn test_no_skip_file() {
        let code = r#"
# Just a regular comment
any(is.na(x))
"#;

        let parsed = parse(code, RParserOptions::default());
        let manager = SuppressionManager::from_node(&parsed.syntax());

        // Should not skip the file
        assert!(!manager.should_skip_file(&parsed.syntax()));
    }
}
