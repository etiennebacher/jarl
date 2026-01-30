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

use crate::directive::{DirectiveParseResult, LintDirective, parse_comment_directive};
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

/// Intermediate state used during single-pass comment collection
struct CommentCollector {
    /// Track start positions per rule for building skip regions
    starts: HashMap<Rule, TextRange>,
    /// Completed skip regions
    skip_regions: Vec<SkipRegion>,
    /// Rules suppressed at file level
    file_suppressions: HashSet<Rule>,
    /// Blanket suppression locations
    blanket_suppressions: Vec<TextRange>,
    /// Suppressions with missing explanations
    unexplained_suppressions: Vec<TextRange>,
    /// Misplaced file-level suppressions (not at top of file)
    misplaced_file_suppressions: Vec<TextRange>,
    /// Whether any valid directive was found (for fast path)
    has_any_valid_directive: bool,
}

impl CommentCollector {
    fn new() -> Self {
        Self {
            starts: HashMap::new(),
            skip_regions: Vec::new(),
            file_suppressions: HashSet::new(),
            blanket_suppressions: Vec::new(),
            unexplained_suppressions: Vec::new(),
            misplaced_file_suppressions: Vec::new(),
            has_any_valid_directive: false,
        }
    }
}

/// Tracks which nodes should skip linting based on comments
#[derive(Debug)]
pub struct SuppressionManager {
    pub comments: Comments<RLanguage>,
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
    /// Locations of blanket suppression comments (e.g., `# jarl-ignore` without a rule)
    pub blanket_suppressions: Vec<TextRange>,
    /// Suppressions with missing explanations
    pub unexplained_suppressions: Vec<TextRange>,
    /// Misplaced file-level suppressions (not at top of file)
    pub misplaced_file_suppressions: Vec<TextRange>,
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
                blanket_suppressions: Vec::new(),
                unexplained_suppressions: Vec::new(),
                misplaced_file_suppressions: Vec::new(),
            };
        }

        // Only do expensive comment processing if needed
        let comments = Comments::from_node(root, &RCommentStyle, None);

        // Single pass: collect all directive information at once
        let mut collector = CommentCollector::new();
        Self::collect_all_directives(root, &comments, &mut collector, true);

        let has_any_suppressions = !collector.skip_regions.is_empty()
            || !collector.file_suppressions.is_empty()
            || collector.has_any_valid_directive;

        Self {
            comments,
            skip_regions: collector.skip_regions,
            file_suppressions: collector.file_suppressions,
            has_any_suppressions,
            inherited_suppressions: Vec::new(),
            blanket_suppressions: collector.blanket_suppressions,
            unexplained_suppressions: collector.unexplained_suppressions,
            misplaced_file_suppressions: collector.misplaced_file_suppressions,
        }
    }

    /// Single-pass collection of all directive information from comments
    fn collect_all_directives(
        node: &RSyntaxNode,
        comments: &Comments<RLanguage>,
        collector: &mut CommentCollector,
        is_first_expression: bool,
    ) {
        // Process leading comments (file suppressions only valid here)
        for comment in comments.leading_comments(node) {
            Self::process_comment(
                comment.piece().text(),
                comment.piece().text_range(),
                collector,
                is_first_expression,
            );
        }

        // Process trailing comments
        for comment in comments.trailing_comments(node) {
            Self::process_comment(
                comment.piece().text(),
                comment.piece().text_range(),
                collector,
                false,
            );
        }

        // Process dangling comments
        for comment in comments.dangling_comments(node) {
            Self::process_comment(
                comment.piece().text(),
                comment.piece().text_range(),
                collector,
                false,
            );
        }

        // Recursively process children
        let mut is_first = is_first_expression;
        for child in node.children() {
            Self::collect_all_directives(&child, comments, collector, is_first);
            is_first = false; // Only the first child can have file-level suppressions
        }
    }

    /// Process a single comment and update the collector
    fn process_comment(
        text: &str,
        range: TextRange,
        collector: &mut CommentCollector,
        allow_file_suppression: bool,
    ) {
        match parse_comment_directive(text) {
            Some(DirectiveParseResult::Valid(directive)) => {
                collector.has_any_valid_directive = true;
                match directive {
                    LintDirective::IgnoreStart(rule) => {
                        collector.starts.insert(rule, range);
                    }
                    LintDirective::IgnoreEnd(rule) => {
                        if let Some(start_range) = collector.starts.remove(&rule) {
                            collector.skip_regions.push(SkipRegion {
                                range: TextRange::new(start_range.start(), range.end()),
                                rule,
                            });
                        }
                    }
                    LintDirective::IgnoreFile(rule) => {
                        if allow_file_suppression {
                            collector.file_suppressions.insert(rule);
                        } else {
                            collector.misplaced_file_suppressions.push(range);
                        }
                    }
                    LintDirective::Ignore(_) => {
                        // Node-level suppressions are handled at check time via check_node_comments
                    }
                }
            }
            Some(DirectiveParseResult::BlanketSuppression) => {
                collector.blanket_suppressions.push(range);
            }
            Some(DirectiveParseResult::MissingExplanation) => {
                collector.unexplained_suppressions.push(range);
            }
            None => {}
        }
    }

    /// Check suppression directives attached to a single node (not ancestors)
    pub fn check_node_comments(&self, node: &RSyntaxNode) -> HashSet<Rule> {
        let mut suppressed = HashSet::new();

        // Check for node-level directives in leading comments
        for comment in self.comments.leading_comments(node) {
            let text = comment.piece().text();
            if let Some(DirectiveParseResult::Valid(LintDirective::Ignore(rule))) =
                parse_comment_directive(text)
            {
                suppressed.insert(rule);
            }
        }

        // Check trailing comments
        for comment in self.comments.trailing_comments(node) {
            let text = comment.piece().text();
            if let Some(DirectiveParseResult::Valid(LintDirective::Ignore(rule))) =
                parse_comment_directive(text)
            {
                suppressed.insert(rule);
            }
        }

        // Check dangling comments
        for comment in self.comments.dangling_comments(node) {
            let text = comment.piece().text();
            if let Some(DirectiveParseResult::Valid(LintDirective::Ignore(rule))) =
                parse_comment_directive(text)
            {
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
