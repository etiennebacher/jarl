//! Parsing of suppression comment directives.
//!
//! This module handles parsing `# jarl-ignore` comments to determine
//! which rules should be suppressed.

use crate::rule_set::Rule;

/// A parsed lint directive from a comment
#[derive(Debug, PartialEq, Clone)]
pub enum LintDirective {
    /// Skip specific rule for the next node: `# jarl-ignore <rule>: <explanation>`
    Ignore(Rule),
    /// Skip entire file for a rule: `# jarl-ignore-file <rule>: <explanation>`
    IgnoreFile(Rule),
    /// Start a range suppression: `# jarl-ignore-start <rule>: <explanation>`
    IgnoreStart(Rule),
    /// End a range suppression: `# jarl-ignore-end <rule>`
    IgnoreEnd(Rule),
}

/// Result of parsing a comment that looks like a suppression directive.
///
/// This reports valid lint directives but also those that are invalid for any
/// reason (blanket suppression, wrong rule name, no explanation, etc.). We do
/// this here to parse and collect potential errors in comments in a single
/// pass.
///
/// Information on the invalid comments is then reported when we run the checks.
#[derive(Debug, PartialEq, Clone)]
pub enum DirectiveParseResult {
    /// A valid, complete directive
    Valid(LintDirective),
    /// Comment is `# jarl-ignore` without specifying a rule (blanket suppression)
    BlanketSuppression,
}

/// Parse a comment directive
///
/// Supported formats:
///
/// ```text
/// # jarl-ignore <rule>: <explanation>
/// # jarl-ignore-file <rule>: <explanation>
/// # jarl-ignore-start <rule>: <explanation>
/// # jarl-ignore-end <rule>
/// ```
///
/// Also accepts without space after `#`:
/// ```text
/// #jarl-ignore <rule>: <explanation>
/// ```
///
/// Notes:
/// - Rule name must be valid (validated against known rules)
/// - Explanation is mandatory (except for `-end`)
/// - One rule per directive
///
/// Returns:
/// - `Some(Valid(directive))` - A valid directive was found
/// - `Some(BlanketSuppression)` - A blanket suppression was found (no rule specified)
/// - `None` - Not a suppression comment at all
pub fn parse_comment_directive(text: &str) -> Option<DirectiveParseResult> {
    let text = text.trim_start();

    // Accept both "# jarl-ignore" and "#jarl-ignore"
    let text = if let Some(rest) = text.strip_prefix("# ") {
        rest
    } else if let Some(rest) = text.strip_prefix('#') {
        rest
    } else {
        return None;
    };

    // Must start with "jarl-ignore"
    let rest = text.strip_prefix("jarl-ignore")?;

    // Determine the directive type based on what follows
    if let Some(after_file) = rest.strip_prefix("-file ") {
        // `# jarl-ignore-file <rule>: <explanation>`
        parse_rule_with_explanation(after_file)
            .map(|rule| DirectiveParseResult::Valid(LintDirective::IgnoreFile(rule)))
    } else if let Some(after_start) = rest.strip_prefix("-start ") {
        // `# jarl-ignore-start <rule>: <explanation>`
        parse_rule_with_explanation(after_start)
            .map(|rule| DirectiveParseResult::Valid(LintDirective::IgnoreStart(rule)))
    } else if let Some(after_end) = rest.strip_prefix("-end ") {
        // `# jarl-ignore-end <rule>`
        parse_rule_only(after_end)
            .map(|rule| DirectiveParseResult::Valid(LintDirective::IgnoreEnd(rule)))
    } else if let Some(after_ignore) = rest.strip_prefix(' ') {
        // `# jarl-ignore <rule>: <explanation>`
        // If after_ignore starts with `:`, it's a blanket suppression (no rule name)
        if after_ignore.starts_with(':') {
            Some(DirectiveParseResult::BlanketSuppression)
        } else {
            parse_rule_with_explanation(after_ignore)
                .map(|rule| DirectiveParseResult::Valid(LintDirective::Ignore(rule)))
        }
    } else if rest.is_empty() || rest.starts_with(':') {
        // Blanket suppression: `# jarl-ignore`, `#jarl-ignore`, or `# jarl-ignore:`
        Some(DirectiveParseResult::BlanketSuppression)
    } else {
        // Not a valid directive (e.g., `# jarl-ignorefoo`)
        None
    }
}

/// Parse a rule name followed by `: <explanation>`
///
/// Format: `<rule>: <explanation>`
/// - Rule name must be valid
/// - Colon and explanation are mandatory
/// - Explanation must be non-empty
fn parse_rule_with_explanation(text: &str) -> Option<Rule> {
    // Find the colon separator
    let colon_pos = text.find(':')?;

    // Extract and validate rule name
    let rule_name = text[..colon_pos].trim();
    if rule_name.is_empty() {
        return None;
    }

    // Check explanation exists (non-empty after colon)
    let explanation = text[colon_pos + 1..].trim();
    if explanation.is_empty() {
        return None;
    }

    // Validate rule name against known rules
    Rule::from_name(rule_name)
}

/// Parse a rule name only (for `-end` directives)
///
/// Format: `<rule>`
/// - Rule name must be valid
/// - No colon or explanation expected
fn parse_rule_only(text: &str) -> Option<Rule> {
    let rule_name = text.trim();
    if rule_name.is_empty() {
        return None;
    }

    // Validate rule name against known rules
    Rule::from_name(rule_name)
}
