use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for suppression comments that are missing an explanation.
///
/// ## Why is this bad?
///
/// Suppression comments without explanations make it hard to understand why a
/// rule was suppressed. Over time, these unexplained suppressions can lead to
/// technical debt as developers may not know if the suppression is still needed
/// or what the original reason was.
///
/// A `# jarl-ignore` comment without an explanation is ignored by Jarl.
///
/// ## Example
///
/// ```r
/// # The comment below isn't applied, the code below is still reported.
/// # jarl-ignore any_is_na
/// any(is.na(x))
/// ```
///
/// Use instead:
/// ```r
/// # jarl-ignore any_is_na: <reason>
/// any(is.na(x))
/// ```
pub fn unexplained_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "unexplained_suppression".to_string(),
            "This comment isn't used by Jarl because it is missing an explanation.".to_string(),
            Some(
                "Add an explanation after the colon, e.g., `# jarl-ignore rule: <reason>`."
                    .to_string(),
            ),
        ),
        range,
        Fix::empty(),
    )
}
