use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for `# jarl-ignore-file` comments that are not at the top of the file.
///
/// ## Why is this bad?
///
/// File-level suppression comments must appear at the very beginning of the file
/// (before any code) to be applied. A `# jarl-ignore-file` comment placed
/// elsewhere in the file is silently ignored by Jarl.
///
/// ## Example
///
/// ```r
/// x <- 1
///
/// # The comment below isn't applied because it's not at the top of the file.
/// # jarl-ignore-file any_is_na: <explanation>
/// any(is.na(x))
/// ```
///
/// Use instead:
/// ```r
/// # jarl-ignore-file any_is_na: <explanation>
///
/// x <- 1
/// any(is.na(x))
/// ```
pub fn misplaced_file_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "misplaced_file_suppression".to_string(),
            "This comment isn't used by Jarl because `# jarl-ignore-file` must be at the top of the file.".to_string(),
            Some("Move this comment to the beginning of the file, before any code.".to_string()),
        ),
        range,
        Fix::empty(),
    )
}
