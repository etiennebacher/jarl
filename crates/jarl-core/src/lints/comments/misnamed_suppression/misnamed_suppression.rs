use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for suppression comments with an invalid rule name.
///
/// ## Why is this bad?
///
/// A suppression comment with an unrecognized rule name will not suppress any
/// violations. This could be due to a typo in the rule name or using a rule
/// name that doesn't exist.
///
/// ## Example
///
/// ```r
/// # The comment below isn't applied because "any_isna" is not a valid rule.
/// # jarl-ignore any_isna: <reason>
/// any(is.na(x))
/// ```
///
/// Use instead:
/// ```r
/// # jarl-ignore any_is_na: <reason>
/// any(is.na(x))
/// ```
pub fn misnamed_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "misnamed_suppression".to_string(),
            "This comment isn't used by Jarl because it contains an unrecognized rule name."
                .to_string(),
            Some("Check the rule name for typos.".to_string()),
        ),
        range,
        Fix::empty(),
    )
}
