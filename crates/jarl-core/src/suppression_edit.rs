//! Infrastructure for auto-inserting suppression comments.
//!
//! This module provides utilities for computing where to insert `# jarl-ignore`
//! comments and how to format them. It handles "hoisting" - finding the appropriate
//! expression to attach suppression comments to, minimizing accidental suppression
//! of unrelated diagnostics.

use air_r_parser::RParserOptions;
use air_r_syntax::RSyntaxKind;
use biome_rowan::{AstNode, SyntaxNode, TextRange, TextSize};

/// Information about where to insert a suppression comment
#[derive(Debug, Clone)]
pub struct SuppressionInsertPoint {
    /// The byte offset where to insert the comment
    pub offset: usize,
    /// The indentation to use (whitespace prefix)
    pub indent: String,
    /// The line number (0-indexed) where the suppression applies
    pub line: usize,
    /// Whether we need to add a leading newline (when expression is inline with other code)
    pub needs_leading_newline: bool,
}

/// Result of attempting to compute a suppression edit
#[derive(Debug, Clone)]
pub struct SuppressionEdit {
    /// Where to insert the comment
    pub insert_point: SuppressionInsertPoint,
    /// The formatted comment text (including newline)
    pub comment_text: String,
}

/// Format a suppression comment for a specific rule.
///
/// If `needs_leading_newline` is true, the comment will be preceded by a newline
/// and the expression text will follow on a new line with the same indentation.
pub fn format_suppression_comment(
    rule_name: &str,
    explanation: &str,
    indent: &str,
    needs_leading_newline: bool,
) -> String {
    if needs_leading_newline {
        // Insert newline, then indented comment, then newline + indent for the expression
        format!(
            "\n{}# jarl-ignore {}: {}\n{}",
            indent, rule_name, explanation, indent
        )
    } else {
        format!("{}# jarl-ignore {}: {}\n", indent, rule_name, explanation)
    }
}

/// Compute where to insert a suppression comment for a diagnostic at the given byte range.
///
/// This performs minimal hoisting - finding the smallest expression containing the diagnostic
/// that can have a suppression comment placed before it. For inline expressions (those that
/// share a line with other code), it returns an insertion point that includes a leading newline.
///
/// The algorithm prioritizes inserting at line start when possible:
/// 1. First find the smallest meaningful expression containing the diagnostic
/// 2. Walk up to find the smallest expression that starts on its own line
/// 3. If found, insert at that line start (normal case)
/// 4. If no expression is on its own line, insert inline with a leading newline
///
/// For example, take the following code:
///
/// ```r
/// f <- function(
///   x = any(is.na(x))
/// ) {
///   1
/// }
/// ```
///
/// We encounter the violation `any(is.na(x))`. This is a meaningful expression
/// because `R_CALL` is in the list below, *but* it doesn't start on its own
/// line so we go higher in the AST.
///
/// Then we encounter, `x = any(is.na(x))` which is an `R_PARAMETER`. This is
/// also a meaningful expression, *and* it's on its own line so we stop here
/// and insert the comment above this line:
///
/// ```r
/// f <- function(
///   # jarl-ignore any_is_na: <reason>
///   x = any(is.na(x))
/// ) {
///   1
/// }
/// ```
///
/// This algorithm has an exception: control flow nodes. When we meet a control
/// flow node, we stop and use the latest expression found. Take this code for
/// example:
///
/// ```r
/// if (a) {
///   print(1)
/// } else if (any(is.na(b))) {
///   print(2)
/// }
/// ```
///
/// We encounter `any(is.na(b))`, which is a meaningful expression but not on
/// its own line, so we go higher in the AST. When we go up, we find the
/// `R_IF_STATEMENT`, which is a control flow node. If we followed the procedure
/// as initially defined, we would insert the comment above the entire `if` statement.
/// The problem is that this could lead to accidental suppressions in other branches
/// of this statement.
///
/// Therefore, we stop and revert to the previous node found. Since it's not on
/// its own line, we insert a newline before the comment, giving this:
///
/// ```r
/// if (a) {
///   print(1)
/// } else if (
///            # jarl-ignore any_is_na: <reason>
///            any(is.na(b))) {
///   print(2)
/// }
///
/// *Note:* Jarl doesn't format code, this has to be done with a proper
/// formatter like Air.
///
/// # Arguments
/// * `source` - The source code
/// * `diagnostic_start` - The start byte offset of the diagnostic
/// * `diagnostic_end` - The end byte offset of the diagnostic
///
/// # Returns
/// The insertion point if the code can be parsed, None otherwise.
pub fn compute_suppression_insert_point(
    source: &str,
    diagnostic_start: usize,
    diagnostic_end: usize,
) -> Option<SuppressionInsertPoint> {
    let parsed = air_r_parser::parse(source, RParserOptions::default());
    if parsed.has_error() {
        // Fall back to simple line-based insertion
        return compute_simple_insert_point(source, diagnostic_start);
    }

    let root = parsed.tree();

    // Find the diagnostic range
    let diagnostic_range = TextRange::new(
        TextSize::from(diagnostic_start as u32),
        TextSize::from(diagnostic_end as u32),
    );

    // Find the smallest expression containing the diagnostic using covering_element
    let covering = root.syntax().covering_element(diagnostic_range);
    let start_node = match covering {
        biome_rowan::NodeOrToken::Node(node) => node,
        biome_rowan::NodeOrToken::Token(token) => token.parent()?,
    };

    // Walk up the tree to find meaningful expressions
    // We want to find the smallest expression that is on its own line
    // BUT if we encounter a control flow statement that's on its own line,
    // we prefer inline insertion at a smaller meaningful expression to be more targeted
    let mut current = start_node;
    let mut smallest_meaningful: Option<SyntaxNode<air_r_syntax::RLanguage>> = None;

    loop {
        if is_meaningful_expression(&current) {
            let node_start = current.text_trimmed_range().start();
            let node_start_offset: usize = node_start.into();

            // Check if this expression is on its own line
            if is_on_own_line(source, node_start_offset) {
                // If this is a control flow statement and we already have a smaller
                // meaningful expression, prefer inline insertion at the smaller one
                // to avoid accidentally suppressing too much
                if is_control_flow_statement(&current)
                    && let Some(smallest_meaningful) = smallest_meaningful
                {
                    // Use inline insertion at the smaller expression
                    let inline_node = smallest_meaningful;
                    let inline_start = inline_node.text_trimmed_range().start();
                    let inline_offset: usize = inline_start.into();

                    let indent = compute_inline_indent(source, inline_offset);
                    let line_number = count_lines_to(source, inline_offset);

                    return Some(SuppressionInsertPoint {
                        offset: inline_offset,
                        indent,
                        line: line_number,
                        needs_leading_newline: true,
                    });
                }

                // Found a non-control-flow expression on its own line - use this
                let (line_start_offset, indent) =
                    find_line_start_and_indent(source, node_start_offset);
                let line_number = source[..line_start_offset].matches('\n').count();

                return Some(SuppressionInsertPoint {
                    offset: line_start_offset,
                    indent,
                    line: line_number,
                    needs_leading_newline: false,
                });
            }

            // Remember the smallest meaningful expression for inline insertion
            if smallest_meaningful.is_none() {
                smallest_meaningful = Some(current.clone());
            }
        }

        // Go up to parent
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // No expression found on its own line - use inline insertion at the smallest meaningful
    if let Some(inline_node) = smallest_meaningful {
        let node_start = inline_node.text_trimmed_range().start();
        let node_start_offset: usize = node_start.into();

        // Calculate indent based on the column position
        let indent = compute_inline_indent(source, node_start_offset);
        let line_number = count_lines_to(source, node_start_offset);

        return Some(SuppressionInsertPoint {
            offset: node_start_offset,
            indent,
            line: line_number,
            needs_leading_newline: true,
        });
    }

    // Fallback to simple insertion
    compute_simple_insert_point(source, diagnostic_start)
}

/// Check if a position is on its own line (preceded only by whitespace after a newline)
fn is_on_own_line(source: &str, offset: usize) -> bool {
    if offset == 0 {
        return true;
    }

    let before = &source[..offset];
    if let Some(newline_pos) = before.rfind('\n') {
        let line_prefix = &before[newline_pos + 1..];
        line_prefix.chars().all(|c| c.is_whitespace())
    } else {
        // First line - check if it starts at beginning or after only whitespace
        before.chars().all(|c| c.is_whitespace())
    }
}

/// Check if a syntax node is a meaningful expression that can have a suppression
/// comment.
fn is_meaningful_expression(node: &SyntaxNode<air_r_syntax::RLanguage>) -> bool {
    matches!(
        node.kind(),
        RSyntaxKind::R_BINARY_EXPRESSION
            | RSyntaxKind::R_CALL
            | RSyntaxKind::R_IF_STATEMENT
            | RSyntaxKind::R_FOR_STATEMENT
            | RSyntaxKind::R_WHILE_STATEMENT
            | RSyntaxKind::R_REPEAT_STATEMENT
            | RSyntaxKind::R_FUNCTION_DEFINITION
            | RSyntaxKind::R_UNARY_EXPRESSION
            | RSyntaxKind::R_SUBSET
            | RSyntaxKind::R_SUBSET2
            | RSyntaxKind::R_PARAMETER
    )
}

/// Check if a syntax node is a control flow statement that can contain many sub-expressions.
/// We should be more targeted when placing suppressions inside these.
fn is_control_flow_statement(node: &SyntaxNode<air_r_syntax::RLanguage>) -> bool {
    matches!(
        node.kind(),
        RSyntaxKind::R_IF_STATEMENT
            | RSyntaxKind::R_FOR_STATEMENT
            | RSyntaxKind::R_WHILE_STATEMENT
            | RSyntaxKind::R_REPEAT_STATEMENT
            | RSyntaxKind::R_FUNCTION_DEFINITION
    )
}

/// Compute the indentation for an inline insertion
fn compute_inline_indent(source: &str, offset: usize) -> String {
    // Find the column position of the offset
    let before = &source[..offset];
    let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let column = offset - line_start;

    // Use spaces for indentation matching the column
    " ".repeat(column)
}

/// Count the number of lines before an offset
fn count_lines_to(source: &str, offset: usize) -> usize {
    source[..offset].matches('\n').count()
}

/// Simple fallback: compute insertion point based on the diagnostic line
fn compute_simple_insert_point(source: &str, byte_offset: usize) -> Option<SuppressionInsertPoint> {
    let (line_start_offset, indent) = find_line_start_and_indent(source, byte_offset);
    let line_number = source[..line_start_offset].matches('\n').count();

    Some(SuppressionInsertPoint {
        offset: line_start_offset,
        indent,
        line: line_number,
        needs_leading_newline: false,
    })
}

/// Find the start of the line containing the given byte offset, and extract indentation.
fn find_line_start_and_indent(source: &str, byte_offset: usize) -> (usize, String) {
    // Find the start of the line
    let line_start = source[..byte_offset]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);

    // Extract indentation (leading whitespace)
    let line_text = &source[line_start..];
    let indent: String = line_text
        .chars()
        .take_while(|c| c.is_whitespace() && *c != '\n')
        .collect();

    (line_start, indent)
}

/// Check if a line contains a jarl-ignore comment and return its parts if so.
/// Returns (indent, rule_names) where rule_names is None for blanket suppression.
pub fn parse_existing_suppression(line: &str) -> Option<(String, Option<Vec<String>>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("# jarl-ignore") {
        return None;
    }

    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();

    // Check for blanket suppression
    let after_prefix = trimmed.strip_prefix("# jarl-ignore")?;
    let after_prefix = after_prefix.trim_start();

    if after_prefix.is_empty() {
        // Blanket suppression: "# jarl-ignore"
        return Some((indent, None));
    }

    // Parse rule name(s) - format: "rule: explanation" or "rule1, rule2: explanation"
    if let Some(colon_pos) = after_prefix.find(':') {
        let rules_part = &after_prefix[..colon_pos];
        let rules: Vec<String> = rules_part
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !rules.is_empty() {
            return Some((indent, Some(rules)));
        }
    }

    None
}

/// Create a complete suppression edit for a diagnostic.
///
/// This is the main entry point for creating suppression comments.
pub fn create_suppression_edit(
    source: &str,
    diagnostic_start: usize,
    diagnostic_end: usize,
    rule_name: &str,
    explanation: &str,
) -> Option<SuppressionEdit> {
    let insert_point = compute_suppression_insert_point(source, diagnostic_start, diagnostic_end)?;
    let comment_text = format_suppression_comment(
        rule_name,
        explanation,
        &insert_point.indent,
        insert_point.needs_leading_newline,
    );

    Some(SuppressionEdit { insert_point, comment_text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_suppression_comment() {
        // Without leading newline
        assert_eq!(
            format_suppression_comment("any_is_na", "reason", "", false),
            "# jarl-ignore any_is_na: reason\n"
        );
        assert_eq!(
            format_suppression_comment("any_is_na", "reason", "  ", false),
            "  # jarl-ignore any_is_na: reason\n"
        );

        // With leading newline (for inline expressions)
        assert_eq!(
            format_suppression_comment("any_is_na", "reason", "  ", true),
            "\n  # jarl-ignore any_is_na: reason\n  "
        );
    }

    #[test]
    fn test_compute_insert_point_simple() {
        let source = "x <- 1\nany(is.na(y))\nz <- 3";
        // Diagnostic on "is.na(y)" which starts at position 11 (after "any(")
        let insert = compute_suppression_insert_point(source, 11, 19).unwrap();
        // Should insert at the start of line 1 (0-indexed), before "any(is.na(y))"
        assert_eq!(insert.line, 1);
        assert_eq!(insert.indent, "");
    }

    #[test]
    fn test_compute_insert_point_indented() {
        let source = "f <- function() {\n  any(is.na(x))\n}";
        // Diagnostic on inner expression
        let insert = compute_suppression_insert_point(source, 22, 30).unwrap();
        assert_eq!(insert.indent, "  ");
        assert!(!insert.needs_leading_newline);
    }

    #[test]
    fn test_compute_insert_point_inline_condition() {
        // Test inline expression in if condition: the diagnostic is on "y <- 1"
        // which is NOT on its own line, so we need a leading newline
        let source = "if (x) {\n  1\n} else if (y <- 1) {\n  2\n}";
        // "y <- 1" starts at position 24 (after "} else if (")
        let insert = compute_suppression_insert_point(source, 24, 30).unwrap();

        // Should insert right before "y <- 1" with a leading newline
        assert!(insert.needs_leading_newline);
        assert_eq!(insert.offset, 24); // Right before "y"
    }

    #[test]
    fn test_compute_insert_point_multiline_condition() {
        // Test expression on its own line inside parentheses
        let source = "if (\n  y <- 1\n) {\n  2\n}";
        // "y <- 1" starts at position 6 (after "if (\n  ")
        let insert = compute_suppression_insert_point(source, 6, 12).unwrap();

        // Should insert at the start of the line, no leading newline needed
        assert!(!insert.needs_leading_newline);
        assert_eq!(insert.indent, "  ");
    }

    #[test]
    fn test_parse_existing_suppression() {
        // Blanket suppression
        let (indent, rules) = parse_existing_suppression("# jarl-ignore").unwrap();
        assert_eq!(indent, "");
        assert!(rules.is_none());

        // Single rule
        let (indent, rules) =
            parse_existing_suppression("  # jarl-ignore any_is_na: reason").unwrap();
        assert_eq!(indent, "  ");
        assert_eq!(rules, Some(vec!["any_is_na".to_string()]));

        // Multiple rules
        let (indent, rules) =
            parse_existing_suppression("# jarl-ignore any_is_na, equals_na: reason").unwrap();
        assert_eq!(indent, "");
        assert_eq!(
            rules,
            Some(vec!["any_is_na".to_string(), "equals_na".to_string()])
        );

        // Not a suppression
        assert!(parse_existing_suppression("# some other comment").is_none());
    }
}
