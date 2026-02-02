use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for blanket suppression comments. Those are comments such as
/// `# jarl-ignore: <reason>` where a rule isn't specified.
///
/// ## Why is this bad?
///
/// This type of comment isn't supported by Jarl as it would suppress all
/// possible violations. Suppression comments should always target one or a few
/// rules, but never all of them.
///
/// ## Example
///
/// ```r
/// # The comment below isn't applied, the code below is still reported.
/// # jarl-ignore: <reason>
/// any(is.na(x))
/// ```
///
/// Use instead to ignore the violation:
/// ```r
/// # jarl-ignore any_is_na: <reason>
/// any(is.na(x))
/// ```
pub fn blanket_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "blanket_suppression".to_string(),
            "This comment isn't used by Jarl because it is missing a rule to ignore.".to_string(),
            Some(
                "Use targeted comments instead, e.g., `# jarl-ignore any_is_na: <reason>`."
                    .to_string(),
            ),
        ),
        range,
        Fix::empty(),
    )
}
