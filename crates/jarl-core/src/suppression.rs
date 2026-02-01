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

use crate::diagnostic::Diagnostic;
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
        // Handle comments inside function call arguments to ensure suppression
        // comments work correctly. Without this special handling, the comment
        // would be attached to R_CALL_ARGUMENTS or R_ARGUMENT_LIST instead of
        // the actual expression, meaning "# jarl-ignore" would be useless.
        //
        // Example:
        // ```
        // foo(
        //   # jarl-ignore rule: reason
        //   any(is.na(x))
        // )
        // ```
        let enclosing = comment.enclosing_node();
        if enclosing.kind() == RSyntaxKind::R_CALL_ARGUMENTS
            || enclosing.kind() == RSyntaxKind::R_ARGUMENT_LIST
        {
            // Comment is inside function call arguments. Attach to the following
            // argument's first child (the actual expression).
            //
            // Example:
            // ```
            // foo(
            //   first_arg,
            //   # jarl-ignore rule: reason
            //   second_arg
            // )
            // ```
            if let Some(following) = comment.following_node()
                && following.kind() == RSyntaxKind::R_ARGUMENT
                && let Some(first_child) = following.first_child()
            {
                return CommentPlacement::leading(first_child, comment);
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
    /// The range of the comment that created this region (for tracking usage)
    pub comment_range: TextRange,
}

/// Represents a node-level suppression (# jarl-ignore rule: explanation)
#[derive(Debug, Clone)]
pub struct NodeSuppression {
    /// The range of the node this suppression applies to
    pub node_range: TextRange,
    /// The rule being suppressed
    pub rule: Rule,
    /// The range of the suppression comment
    pub comment_range: TextRange,
}

/// Represents a file-level suppression
#[derive(Debug, Clone)]
pub struct FileSuppression {
    /// The rule being suppressed
    pub rule: Rule,
    /// The range of the suppression comment
    pub comment_range: TextRange,
}

/// Intermediate state used during single-pass comment collection
struct CommentCollector {
    /// Track start positions per (rule, nesting_level) for building skip regions
    /// Key: (rule, nesting_level), Value: (region_start_range, comment_range)
    starts: HashMap<(Rule, usize), (TextRange, TextRange)>,
    /// Completed skip regions
    skip_regions: Vec<SkipRegion>,
    /// Rules suppressed at file level
    file_suppressions: Vec<FileSuppression>,
    /// Node-level suppressions
    node_suppressions: Vec<NodeSuppression>,
    /// Blanket suppression locations
    blanket_suppressions: Vec<TextRange>,
    /// Suppressions with missing explanations
    unexplained_suppressions: Vec<TextRange>,
    /// Misplaced file-level suppressions (not at top of file)
    misplaced_file_suppressions: Vec<TextRange>,
    /// End-of-line suppression comments (trailing comments)
    misplaced_suppressions: Vec<TextRange>,
    /// Suppressions with invalid rule names
    misnamed_suppressions: Vec<TextRange>,
    /// Unmatched start suppressions (no matching end at the same nesting level)
    unmatched_start_suppressions: Vec<TextRange>,
    /// Unmatched end suppressions (no matching start at the same nesting level)
    unmatched_end_suppressions: Vec<TextRange>,
    /// Whether any valid directive was found (for fast path)
    has_any_valid_directive: bool,
}

impl CommentCollector {
    fn new() -> Self {
        Self {
            starts: HashMap::new(),
            skip_regions: Vec::new(),
            file_suppressions: Vec::new(),
            node_suppressions: Vec::new(),
            blanket_suppressions: Vec::new(),
            unexplained_suppressions: Vec::new(),
            misplaced_file_suppressions: Vec::new(),
            misplaced_suppressions: Vec::new(),
            misnamed_suppressions: Vec::new(),
            unmatched_start_suppressions: Vec::new(),
            unmatched_end_suppressions: Vec::new(),
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
    pub file_suppressions: Vec<FileSuppression>,
    /// Node-level suppressions (# jarl-ignore rule: explanation)
    pub node_suppressions: Vec<NodeSuppression>,
    /// Fast path: true if there are any suppressions anywhere in the file
    pub has_any_suppressions: bool,
    /// Locations of blanket suppression comments (e.g., `# jarl-ignore` without a rule)
    pub blanket_suppressions: Vec<TextRange>,
    /// Unmatched start suppressions (no matching end at the same nesting level)
    pub unmatched_start_suppressions: Vec<TextRange>,
    /// Unmatched end suppressions (no matching start at the same nesting level)
    pub unmatched_end_suppressions: Vec<TextRange>,
    /// Suppressions with missing explanations
    pub unexplained_suppressions: Vec<TextRange>,
    /// Misplaced file-level suppressions (not at top of file)
    pub misplaced_file_suppressions: Vec<TextRange>,
    /// End-of-line suppression comments (trailing comments)
    pub misplaced_suppressions: Vec<TextRange>,
    /// Suppressions with invalid rule names
    pub misnamed_suppressions: Vec<TextRange>,
    /// Tracks which suppression comment ranges have been used (suppressed a real violation)
    pub used_suppressions: HashSet<TextRange>,
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
                file_suppressions: Vec::new(),
                node_suppressions: Vec::new(),
                has_any_suppressions: false,
                blanket_suppressions: Vec::new(),
                unmatched_start_suppressions: Vec::new(),
                unmatched_end_suppressions: Vec::new(),
                unexplained_suppressions: Vec::new(),
                misplaced_file_suppressions: Vec::new(),
                misplaced_suppressions: Vec::new(),
                misnamed_suppressions: Vec::new(),
                used_suppressions: HashSet::new(),
            };
        }

        // Only do expensive comment processing if needed
        let comments = Comments::from_node(root, &RCommentStyle, None);

        // Single pass: collect all directive information at once
        let mut collector = CommentCollector::new();
        Self::collect_all_directives(root, &comments, &mut collector, true, source, 0);

        // Any remaining starts without matching ends are unmatched
        for ((_, _), (comment_range, _)) in collector.starts.drain() {
            collector.unmatched_start_suppressions.push(comment_range);
        }

        let has_any_suppressions = !collector.skip_regions.is_empty()
            || !collector.file_suppressions.is_empty()
            || collector.has_any_valid_directive;

        Self {
            comments,
            skip_regions: collector.skip_regions,
            file_suppressions: collector.file_suppressions,
            node_suppressions: collector.node_suppressions,
            has_any_suppressions,
            blanket_suppressions: collector.blanket_suppressions,
            unmatched_start_suppressions: collector.unmatched_start_suppressions,
            unmatched_end_suppressions: collector.unmatched_end_suppressions,
            unexplained_suppressions: collector.unexplained_suppressions,
            misplaced_file_suppressions: collector.misplaced_file_suppressions,
            misplaced_suppressions: collector.misplaced_suppressions,
            misnamed_suppressions: collector.misnamed_suppressions,
            used_suppressions: HashSet::new(),
        }
    }

    /// Single-pass collection of all directive information from comments
    fn collect_all_directives(
        node: &RSyntaxNode,
        comments: &Comments<RLanguage>,
        collector: &mut CommentCollector,
        is_first_expression: bool,
        source: &str,
        nesting_level: usize,
    ) {
        let node_range = node.text_trimmed_range();

        // Process leading comments (file suppressions only valid here)
        for comment in comments.leading_comments(node) {
            Self::process_comment(
                comment.piece().text(),
                comment.piece().text_range(),
                node_range,
                collector,
                is_first_expression,
                false, // not trailing
                nesting_level,
            );
        }

        // Process trailing comments (end-of-line comments)
        // Note: biome classifies comments at EOF as "trailing" even if they're on their own line.
        // We need to check if the comment is actually on the same line as code (true end-of-line).
        for comment in comments.trailing_comments(node) {
            let range = comment.piece().text_range();
            let is_true_end_of_line = Self::is_same_line_as_code(range, source);
            Self::process_comment(
                comment.piece().text(),
                range,
                node_range,
                collector,
                false,
                is_true_end_of_line,
                nesting_level,
            );
        }

        // Process dangling comments
        for comment in comments.dangling_comments(node) {
            Self::process_comment(
                comment.piece().text(),
                comment.piece().text_range(),
                node_range,
                collector,
                false,
                false, // not trailing
                nesting_level,
            );
        }

        // Recursively process children
        // Increment nesting level when entering braced expressions (function bodies, etc.)
        let mut is_first = is_first_expression;
        for child in node.children() {
            let child_nesting = if child.kind() == RSyntaxKind::R_BRACED_EXPRESSIONS {
                nesting_level + 1
            } else {
                nesting_level
            };
            Self::collect_all_directives(
                &child,
                comments,
                collector,
                is_first,
                source,
                child_nesting,
            );
            is_first = false; // Only the first child can have file-level suppressions
        }
    }

    /// Check if a comment is on the same line as code (true end-of-line comment)
    /// Returns true if there's no newline between the start of the line and the comment
    fn is_same_line_as_code(comment_range: TextRange, source: &str) -> bool {
        let start = comment_range.start().into();
        if start == 0 {
            return false; // Comment at start of file is not end-of-line
        }

        // Look backwards from comment start to find if there's code on the same line
        let before_comment = &source[..start];

        // Find the last newline before the comment
        if let Some(last_newline) = before_comment.rfind('\n') {
            // Check if there's any non-whitespace between the newline and the comment
            let between = &before_comment[last_newline + 1..];
            between.chars().any(|c| !c.is_whitespace())
        } else {
            // No newline found - comment is on first line
            // Check if there's any non-whitespace before it
            before_comment.chars().any(|c| !c.is_whitespace())
        }
    }

    /// Process a single comment and update the collector
    fn process_comment(
        text: &str,
        comment_range: TextRange,
        node_range: TextRange,
        collector: &mut CommentCollector,
        allow_file_suppression: bool,
        is_trailing: bool,
        nesting_level: usize,
    ) {
        match parse_comment_directive(text) {
            Some(DirectiveParseResult::Valid(directive)) => {
                // Trailing comments (end-of-line) are not supported for suppressions
                if is_trailing {
                    collector.misplaced_suppressions.push(comment_range);
                    return;
                }
                collector.has_any_valid_directive = true;
                match directive {
                    LintDirective::IgnoreStart(rule) => {
                        // Store with nesting level for proper matching
                        collector
                            .starts
                            .insert((rule, nesting_level), (comment_range, comment_range));
                    }
                    LintDirective::IgnoreEnd(rule) => {
                        // Only match with start at the same nesting level
                        if let Some((start_comment_range, _)) =
                            collector.starts.remove(&(rule, nesting_level))
                        {
                            collector.skip_regions.push(SkipRegion {
                                range: TextRange::new(
                                    start_comment_range.start(),
                                    comment_range.end(),
                                ),
                                rule,
                                comment_range: start_comment_range,
                            });
                        } else {
                            // No matching start at this nesting level
                            collector.unmatched_end_suppressions.push(comment_range);
                        }
                    }
                    LintDirective::IgnoreFile(rule) => {
                        if allow_file_suppression {
                            collector
                                .file_suppressions
                                .push(FileSuppression { rule, comment_range });
                        } else {
                            collector.misplaced_file_suppressions.push(comment_range);
                        }
                    }
                    LintDirective::Ignore(rule) => {
                        // Collect node-level suppressions
                        collector.node_suppressions.push(NodeSuppression {
                            node_range,
                            rule,
                            comment_range,
                        });
                    }
                }
            }
            Some(DirectiveParseResult::BlanketSuppression) => {
                // Trailing comments are also misplaced
                if is_trailing {
                    collector.misplaced_suppressions.push(comment_range);
                } else {
                    collector.blanket_suppressions.push(comment_range);
                }
            }
            Some(DirectiveParseResult::MissingExplanation) => {
                // Trailing comments are also misplaced
                if is_trailing {
                    collector.misplaced_suppressions.push(comment_range);
                } else {
                    collector.unexplained_suppressions.push(comment_range);
                }
            }
            Some(DirectiveParseResult::InvalidRuleName) => {
                // Trailing comments are also misplaced
                if is_trailing {
                    collector.misplaced_suppressions.push(comment_range);
                } else {
                    collector.misnamed_suppressions.push(comment_range);
                }
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
        if self.file_suppressions.iter().any(|s| s.rule == rule) {
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

    /// Filter diagnostics by suppressions and track which suppressions were used.
    /// Returns the filtered diagnostics (those that should be reported).
    ///
    /// This follows Ruff's approach: collect all diagnostics first, then remove
    /// those that are suppressed.
    pub fn filter_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        // Fast path: if there are no suppressions, return all diagnostics
        if !self.has_any_suppressions {
            return diagnostics;
        }

        diagnostics
            .into_iter()
            .filter(|diag| !self.is_diagnostic_suppressed(diag))
            .collect()
    }

    /// Check if a diagnostic should be suppressed, and if so, mark the suppression as used.
    fn is_diagnostic_suppressed(&mut self, diag: &Diagnostic) -> bool {
        let Some(rule) = Rule::from_name(&diag.message.name) else {
            return false;
        };

        // Check file-level suppressions
        for sup in &self.file_suppressions {
            if sup.rule == rule {
                self.used_suppressions.insert(sup.comment_range);
                return true;
            }
        }

        // Check region-level suppressions
        for region in &self.skip_regions {
            if region.rule == rule && region.range.contains_range(diag.range) {
                self.used_suppressions.insert(region.comment_range);
                return true;
            }
        }

        // Check node-level suppressions (cascading: diagnostic within node range)
        for sup in &self.node_suppressions {
            if sup.rule == rule && sup.node_range.contains_range(diag.range) {
                self.used_suppressions.insert(sup.comment_range);
                return true;
            }
        }

        false
    }

    /// Get all suppression comment ranges that were never used.
    /// This is used to report outdated suppressions.
    pub fn get_unused_suppressions(&self) -> Vec<TextRange> {
        let mut unused = Vec::new();

        // Check file-level suppressions
        for sup in &self.file_suppressions {
            if !self.used_suppressions.contains(&sup.comment_range) {
                unused.push(sup.comment_range);
            }
        }

        // Check region-level suppressions
        for region in &self.skip_regions {
            if !self.used_suppressions.contains(&region.comment_range) {
                unused.push(region.comment_range);
            }
        }

        // Check node-level suppressions
        for sup in &self.node_suppressions {
            if !self.used_suppressions.contains(&sup.comment_range) {
                unused.push(sup.comment_range);
            }
        }

        unused
    }
}
