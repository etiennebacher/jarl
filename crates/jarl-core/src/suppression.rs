//! Comment-based suppression for lint rules
//!
//! This module handles extracting and checking `# jarl-ignore` comments
//! to determine which nodes should skip linting.

use air_r_syntax::{RLanguage, RSyntaxKind, RSyntaxNode};
use biome_formatter::comments::{
    CommentKind, CommentPlacement, CommentStyle, Comments, DecoratedComment,
};
use biome_rowan::{SyntaxTriviaPieceComments, TextRange};
use std::collections::{HashMap, HashSet};

use crate::directive::{LintDirective, parse_comment_directive};
use crate::rule_set::Rule;

/// Comment style for R that identifies suppression directives
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
        // of R_CALL, meaning that "# jarl-ignore" is useless:
        // ```
        // foo(
        //   # jarl-ignore rule: reason
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

/// Represents a region where a specific rule should be skipped
#[derive(Debug, Clone)]
pub struct SkipRegion {
    /// The range of text covered by this skip region
    pub range: TextRange,
    /// The rule to skip in this region
    pub rule: Rule,
}

/// Tracks which nodes should skip linting based on comments
#[derive(Debug)]
pub struct SuppressionManager {
    comments: Comments<RLanguage>,
    /// Regions defined by jarl-ignore-start/end blocks
    pub skip_regions: Vec<SkipRegion>,
    /// Rules suppressed at file level via jarl-ignore-file
    pub file_suppressions: HashSet<Rule>,
    /// Fast path: true if there are any suppressions anywhere in the file
    pub has_any_suppressions: bool,
    /// Suppressions inherited from ancestor nodes (for cascading behavior).
    /// This is a stack - we push when entering nodes with suppressions and
    /// truncate when leaving.
    pub inherited_suppressions: Vec<Rule>,
}

impl SuppressionManager {
    /// Create a new suppression manager from the root syntax node
    ///
    /// # Arguments
    /// * `root` - The root syntax node
    /// * `source` - The source code text (used for fast path optimization)
    pub fn from_node(root: &RSyntaxNode, source: &str) -> Self {
        // Fast path: if there's no "jarl-ignore" text anywhere in the source,
        // skip all expensive comment processing
        if !source.contains("jarl-ignore") {
            return Self {
                comments: Comments::default(),
                skip_regions: Vec::new(),
                file_suppressions: HashSet::new(),
                has_any_suppressions: false,
                inherited_suppressions: Vec::new(),
            };
        }

        // Only do expensive comment processing if needed
        let comments = Comments::from_node(root, &RCommentStyle, None);
        let skip_regions = Self::build_skip_regions(root, &comments);
        let file_suppressions = Self::build_file_suppressions(root, &comments);

        // Check if there are any suppressions at all
        let has_any_suppressions = !skip_regions.is_empty()
            || !file_suppressions.is_empty()
            || Self::has_any_directives(root, &comments);

        Self {
            comments,
            skip_regions,
            file_suppressions,
            has_any_suppressions,
            inherited_suppressions: Vec::new(),
        }
    }

    /// Check if there are any jarl-ignore directives in comments
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

    /// Build file-level suppressions from jarl-ignore-file directives
    ///
    /// These must appear in leading comments of the first expression
    /// (before any R code, but can be after other comments).
    fn build_file_suppressions(
        root: &RSyntaxNode,
        comments: &Comments<RLanguage>,
    ) -> HashSet<Rule> {
        let mut suppressions = HashSet::new();

        // A root node always has children, even if the file is empty.
        // We want to check only the leading comments of the first child
        // of the root node (the expression list's first item).
        let r_expression_list = root.first_child().unwrap();
        let first_child = r_expression_list.first_child();

        if let Some(child) = first_child {
            for comment in comments.leading_comments(&child) {
                let text = comment.piece().text();
                if let Some(LintDirective::IgnoreFile(rule)) = parse_comment_directive(text) {
                    suppressions.insert(rule);
                }
            }
        }

        suppressions
    }

    /// Build skip regions from jarl-ignore-start/end directives
    fn build_skip_regions(root: &RSyntaxNode, comments: &Comments<RLanguage>) -> Vec<SkipRegion> {
        let mut regions = Vec::new();
        // Track start positions per rule
        let mut starts: HashMap<Rule, TextRange> = HashMap::new();

        Self::collect_start_end_directives(root, comments, &mut starts, &mut regions);

        regions
    }

    fn collect_start_end_directives(
        node: &RSyntaxNode,
        comments: &Comments<RLanguage>,
        starts: &mut HashMap<Rule, TextRange>,
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
                Some(LintDirective::IgnoreStart(rule)) => {
                    // Start skipping this rule (overwrites previous start if any)
                    starts.insert(rule, range);
                }
                Some(LintDirective::IgnoreEnd(rule)) => {
                    // End the skip region for this rule if there was a matching start
                    if let Some(start_range) = starts.remove(&rule) {
                        regions.push(SkipRegion {
                            range: TextRange::new(start_range.start(), range.end()),
                            rule,
                        });
                    }
                    // Unmatched ends are silently ignored
                }
                _ => {}
            }
        }

        // Recursively process children
        for child in node.children() {
            Self::collect_start_end_directives(&child, comments, starts, regions);
        }
    }

    /// Check suppression directives attached to a single node (not ancestors)
    pub fn check_node_comments(&self, node: &RSyntaxNode) -> HashSet<Rule> {
        let mut suppressed = HashSet::new();

        // Check for node-level directives in leading comments
        for comment in self.comments.leading_comments(node) {
            let text = comment.piece().text();
            if let Some(LintDirective::Ignore(rule)) = parse_comment_directive(text) {
                suppressed.insert(rule);
            }
        }

        // Check trailing comments
        for comment in self.comments.trailing_comments(node) {
            let text = comment.piece().text();
            if let Some(LintDirective::Ignore(rule)) = parse_comment_directive(text) {
                suppressed.insert(rule);
            }
        }

        // Check dangling comments
        for comment in self.comments.dangling_comments(node) {
            let text = comment.piece().text();
            if let Some(LintDirective::Ignore(rule)) = parse_comment_directive(text) {
                suppressed.insert(rule);
            }
        }

        suppressed
    }

    /// Check if a node should skip specific rules
    ///
    /// Returns the set of rules that should be skipped for this node.
    /// Note: This only checks region and node-level suppressions.
    /// Cascading (ancestor) suppressions are handled during AST traversal
    /// in check.rs via inherited_suppressions.
    pub fn check_suppression(&self, node: &RSyntaxNode) -> HashSet<Rule> {
        let mut suppressed = HashSet::new();

        // Check if node is in any skip regions
        let node_range = node.text_trimmed_range();
        for region in &self.skip_regions {
            if region.range.contains_range(node_range) {
                suppressed.insert(region.rule);
            }
        }

        // Check this node's comments only (no ancestor walking)
        suppressed.extend(self.check_node_comments(node));

        suppressed
    }

    /// Check if a specific rule should be skipped for this node
    pub fn should_skip_rule(&self, node: &RSyntaxNode, rule: Rule) -> bool {
        // Fast path: if there are no suppressions anywhere, return immediately
        if !self.has_any_suppressions {
            return false;
        }

        // Check file-level suppressions first
        if self.file_suppressions.contains(&rule) {
            return true;
        }

        // Check skip regions
        let node_range = node.text_trimmed_range();
        for region in &self.skip_regions {
            if region.rule == rule && region.range.contains_range(node_range) {
                return true;
            }
        }

        // Check node-level suppression (with cascading to ancestors)
        let suppressed = self.check_suppression(node);
        suppressed.contains(&rule)
    }
}
