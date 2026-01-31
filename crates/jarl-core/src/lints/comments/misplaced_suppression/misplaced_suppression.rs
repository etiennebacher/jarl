use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for suppression comments placed at the end of a line.
///
/// ## Why is this bad?
///
/// End-of-line suppression comments (trailing comments) are not supported by
/// Jarl because the comment system attaches them to the expression they follow,
/// not to the next expression. This means the suppression would not apply to
/// the intended code.
///
/// ## Example
///
/// ```r
/// # The comment below isn't applied because it's at the end of a line.
/// any(is.na(x)) # jarl-ignore any_is_na: <reason>
/// ```
///
/// Use instead:
/// ```r
/// # jarl-ignore any_is_na: <reason>
/// any(is.na(x))
/// ```
pub fn misplaced_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "misplaced_suppression".to_string(),
            "This comment isn't used by Jarl because end-of-line suppressions are not supported."
                .to_string(),
            Some(
                "Move the suppression comment to its own line above the code you want to suppress."
                    .to_string(),
            ),
        ),
        range,
        Fix::empty(),
    )
}
