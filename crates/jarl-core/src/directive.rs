//! Parsing of suppression comment directives.
//!
//! This module handles parsing `# jarl-ignore` comments to determine
//! which rules should be suppressed.

use crate::rule_set::Rule;

/// A parsed lint directive from a comment
#[derive(Debug, PartialEq, Clone)]
pub enum LintDirective {
    /// Skip specific rule for the next node: `# jarl-ignore <rule>: <reason>`
    Ignore(Rule),
    /// Skip entire chunk for a rule: `#| jarl-ignore-chunk <rule>: <reason>`
    ///
    /// In Rmd/Qmd files this suppresses the rule for the entire chunk (not just
    /// the next expression). In plain `.R` files it behaves like `IgnoreFile`.
    IgnoreChunk(Rule),
    /// Skip entire file for a rule: `# jarl-ignore-file <rule>: <reason>`
    IgnoreFile(Rule),
    /// Start a range suppression: `# jarl-ignore-start <rule>: <reason>`
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
    /// Rule is valid but explanation is missing (no colon or empty after colon)
    MissingExplanation,
    /// Rule name is not recognized
    InvalidRuleName,
}

/// Check whether a comment is the opening line of a Quarto YAML array block
/// for `jarl-ignore-chunk`.
///
/// The Quarto-idiomatic multi-line format is:
///
/// ```text
/// #| jarl-ignore-chunk:
/// #|   - any_is_na: reason
/// #|   - any_duplicated: reason
/// ```
///
/// This function matches the header line `#| jarl-ignore-chunk:` (with nothing
/// meaningful after the colon).  The items are parsed separately by
/// [`parse_quarto_chunk_array_item`].
pub fn is_quarto_chunk_array_header(text: &str) -> bool {
    text.trim() == "#| jarl-ignore-chunk:"
}

/// Parse one item of a Quarto YAML array block for `jarl-ignore-chunk`.
///
/// Matches lines of the form `#|   - <rule>: <reason>` (the `#|` prefix
/// followed by any amount of whitespace, a `-` list marker, whitespace, and
/// then the rule/reason pair).
///
/// Returns:
/// - `Some(Valid(IgnoreChunk(rule)))` — valid item
/// - `Some(MissingExplanation)` — rule recognised but no reason supplied
/// - `Some(InvalidRuleName)` — text looks like an item but rule is unknown
/// - `None` — not a YAML array item (stops the look-ahead)
pub fn parse_quarto_chunk_array_item(text: &str) -> Option<DirectiveParseResult> {
    let text = text.trim();
    // Must start with "#|"
    let rest = text.strip_prefix("#|")?;
    // Trim leading whitespace after "#|"
    let rest = rest.trim_start();
    // Must start with "-" (YAML list item marker)
    let rest = rest.strip_prefix('-')?;
    // Must be followed by whitespace or end-of-string
    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return None;
    }
    let rest = rest.trim_start();
    if rest.is_empty() {
        return None;
    }
    // `rest` is now "<rule>: <reason>"
    Some(match parse_rule_with_explanation(rest) {
        RuleParseResult::Valid(rule) => {
            DirectiveParseResult::Valid(LintDirective::IgnoreChunk(rule))
        }
        RuleParseResult::MissingExplanation => DirectiveParseResult::MissingExplanation,
        RuleParseResult::InvalidRuleName => DirectiveParseResult::InvalidRuleName,
        RuleParseResult::Invalid => return None,
    })
}

/// Parse a comment directive
///
/// Supported formats:
///
/// ```text
/// # jarl-ignore <rule>: <reason>
/// # jarl-ignore-file <rule>: <reason>
/// # jarl-ignore-start <rule>: <reason>
/// # jarl-ignore-end <rule>
/// ```
///
/// In Quarto and R Markdown, `#|` marks a YAML chunk option, so it is only
/// recognised by jarl for `jarl-ignore-chunk`.  All other directives
/// (`jarl-ignore`, `jarl-ignore-file`, `jarl-ignore-start`,
/// `jarl-ignore-end`) must use a plain `#` comment prefix.
///
/// For suppressing a rule across an entire Quarto/Rmd chunk, use the YAML
/// array form (handled by [`is_quarto_chunk_array_header`] /
/// [`parse_quarto_chunk_array_item`]):
///
/// ```text
/// #| jarl-ignore-chunk:
/// #|   - <rule>: <reason>
/// #|   - <rule>: <reason>
/// ```
///
/// The plain-comment form `# jarl-ignore-chunk <rule>: <reason>` is also
/// accepted (without the `#|` prefix).
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

    // Detect the comment prefix.  The `#|` form is a Quarto YAML chunk option
    // and is only recognised for `jarl-ignore-chunk`; all other directives
    // require a plain `#` or `# ` prefix.
    let (text, is_quarto_pipe) = if let Some(rest) = text.strip_prefix("#| ") {
        (rest, true)
    } else if let Some(rest) = text.strip_prefix("# ") {
        (rest, false)
    } else if let Some(rest) = text.strip_prefix('#') {
        (rest, false)
    } else {
        return None;
    };

    // Must start with "jarl-ignore"
    let rest = text.strip_prefix("jarl-ignore")?;

    // Determine the directive type based on what follows.
    // Non-chunk directives are not valid with the `#|` prefix.
    if let Some(after_file) = rest.strip_prefix("-file ") {
        if is_quarto_pipe {
            return None;
        }
        // `# jarl-ignore-file <rule>: <reason>`
        match parse_rule_with_explanation(after_file) {
            RuleParseResult::Valid(rule) => {
                Some(DirectiveParseResult::Valid(LintDirective::IgnoreFile(rule)))
            }
            RuleParseResult::MissingExplanation => Some(DirectiveParseResult::MissingExplanation),
            RuleParseResult::InvalidRuleName => Some(DirectiveParseResult::InvalidRuleName),
            RuleParseResult::Invalid => None,
        }
    } else if let Some(after_start) = rest.strip_prefix("-start ") {
        if is_quarto_pipe {
            return None;
        }
        // `# jarl-ignore-start <rule>: <reason>`
        match parse_rule_with_explanation(after_start) {
            RuleParseResult::Valid(rule) => Some(DirectiveParseResult::Valid(
                LintDirective::IgnoreStart(rule),
            )),
            RuleParseResult::MissingExplanation => Some(DirectiveParseResult::MissingExplanation),
            RuleParseResult::InvalidRuleName => Some(DirectiveParseResult::InvalidRuleName),
            RuleParseResult::Invalid => None,
        }
    } else if let Some(after_end) = rest.strip_prefix("-end ") {
        if is_quarto_pipe {
            return None;
        }
        // `# jarl-ignore-end <rule>` - no explanation required, but tolerate one
        // Strip any explanation (everything after colon) if present
        let rule_part = match after_end.find(':') {
            Some(colon_pos) => &after_end[..colon_pos],
            None => after_end,
        };
        match parse_rule_only(rule_part) {
            Some(rule) => Some(DirectiveParseResult::Valid(LintDirective::IgnoreEnd(rule))),
            None => {
                // Could be invalid rule name or empty - check which
                let rule_name = rule_part.trim();
                if rule_name.is_empty() {
                    None
                } else {
                    Some(DirectiveParseResult::InvalidRuleName)
                }
            }
        }
    } else if let Some(after_ignore) = rest.strip_prefix(' ') {
        if is_quarto_pipe {
            return None;
        }
        // `# jarl-ignore <rule>: <reason>`
        // If after_ignore starts with `:`, it's a blanket suppression (no rule name)
        if after_ignore.starts_with(':') {
            Some(DirectiveParseResult::BlanketSuppression)
        } else {
            match parse_rule_with_explanation(after_ignore) {
                RuleParseResult::Valid(rule) => {
                    Some(DirectiveParseResult::Valid(LintDirective::Ignore(rule)))
                }
                RuleParseResult::MissingExplanation => {
                    Some(DirectiveParseResult::MissingExplanation)
                }
                RuleParseResult::InvalidRuleName => Some(DirectiveParseResult::InvalidRuleName),
                RuleParseResult::Invalid => None,
            }
        }
    } else if rest.is_empty() || rest.starts_with(':') {
        if is_quarto_pipe {
            return None;
        }
        // Blanket suppression: `# jarl-ignore`, `#jarl-ignore`, or `# jarl-ignore:`
        Some(DirectiveParseResult::BlanketSuppression)
    } else if rest == "-chunk" || rest.starts_with("-chunk:") {
        // `#| jarl-ignore-chunk` or `#| jarl-ignore-chunk:` without a rule name
        // → blanket suppression (missing rule).
        Some(DirectiveParseResult::BlanketSuppression)
    } else if let Some(after_chunk) = rest.strip_prefix("-chunk ") {
        // `#| jarl-ignore-chunk <rule>: <reason>` — suppresses the rule for
        // the entire chunk (chunk-wide semantics).
        if after_chunk.starts_with(':') {
            Some(DirectiveParseResult::BlanketSuppression)
        } else {
            match parse_rule_with_explanation(after_chunk) {
                RuleParseResult::Valid(rule) => Some(DirectiveParseResult::Valid(
                    LintDirective::IgnoreChunk(rule),
                )),
                RuleParseResult::MissingExplanation => {
                    Some(DirectiveParseResult::MissingExplanation)
                }
                RuleParseResult::InvalidRuleName => Some(DirectiveParseResult::InvalidRuleName),
                RuleParseResult::Invalid => None,
            }
        }
    } else {
        // Not a valid directive (e.g., `# jarl-ignorefoo`)
        None
    }
}

/// Result of parsing a rule with explanation
enum RuleParseResult {
    /// Valid rule with explanation
    Valid(Rule),
    /// Valid rule but missing explanation
    MissingExplanation,
    /// Rule name is not recognized
    InvalidRuleName,
    /// Invalid (empty rule name or other structural issue)
    Invalid,
}

/// Parse a rule name followed by `: <reason>`
///
/// Format: `<rule>: <reason>`
/// - Rule name must be valid
/// - Colon and explanation are mandatory
/// - Explanation must be non-empty
fn parse_rule_with_explanation(text: &str) -> RuleParseResult {
    // Find the colon separator
    let Some(colon_pos) = text.find(':') else {
        // No colon - check if there's a valid rule name (missing explanation case)
        let rule_name = text.trim();
        if rule_name.is_empty() {
            return RuleParseResult::Invalid;
        }
        return match Rule::from_name(rule_name) {
            Some(_) => RuleParseResult::MissingExplanation,
            None => RuleParseResult::InvalidRuleName,
        };
    };

    // Extract and validate rule name
    let rule_name = text[..colon_pos].trim();
    if rule_name.is_empty() {
        return RuleParseResult::Invalid;
    }

    // Validate rule name against known rules
    let Some(rule) = Rule::from_name(rule_name) else {
        return RuleParseResult::InvalidRuleName;
    };

    // Check explanation exists (non-empty after colon)
    let explanation = text[colon_pos + 1..].trim();
    if explanation.is_empty() {
        return RuleParseResult::MissingExplanation;
    }

    RuleParseResult::Valid(rule)
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
