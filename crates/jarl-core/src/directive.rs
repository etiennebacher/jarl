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
/// - `Some(directive)` - A valid directive was found
/// - `None` - Invalid directive or just a regular comment
pub fn parse_comment_directive(text: &str) -> Option<LintDirective> {
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
        parse_rule_with_explanation(after_file).map(LintDirective::IgnoreFile)
    } else if let Some(after_start) = rest.strip_prefix("-start ") {
        // `# jarl-ignore-start <rule>: <explanation>`
        parse_rule_with_explanation(after_start).map(LintDirective::IgnoreStart)
    } else if let Some(after_end) = rest.strip_prefix("-end ") {
        // `# jarl-ignore-end <rule>`
        parse_rule_only(after_end).map(LintDirective::IgnoreEnd)
    } else if let Some(after_ignore) = rest.strip_prefix(' ') {
        // `# jarl-ignore <rule>: <explanation>`
        parse_rule_with_explanation(after_ignore).map(LintDirective::Ignore)
    } else {
        // Not a valid directive (e.g., `# jarl-ignorefoo` or just `# jarl-ignore`)
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
