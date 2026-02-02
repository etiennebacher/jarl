use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for suppression comments that don't suppress any actual violations.
///
/// ## Why is this bad?
///
/// Suppression comments that are no longer needed can be confusing and may
/// indicate that the underlying code has changed but the comment was not
/// updated. They also add noise to the codebase.
///
/// ## Example
///
/// ```r
/// # The suppression below is unnecessary because there's no any_is_na violation.
/// # jarl-ignore any_is_na: <reason>
/// x <- 1
/// ```
///
/// Use instead:
/// ```r
/// # Remove the suppression comment since it's not needed.
/// x <- 1
/// ```
pub fn outdated_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "outdated_suppression".to_string(),
            "This suppression comment is unused, no violation would be reported without it."
                .to_string(),
            Some("Remove this suppression comment or verify that it's still needed.".to_string()),
        ),
        range,
        Fix::empty(),
    )
}
