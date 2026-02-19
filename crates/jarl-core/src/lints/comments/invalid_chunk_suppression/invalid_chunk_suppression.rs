use crate::diagnostic::*;
use biome_rowan::TextRange;

/// ## What it does
///
/// Checks for `jarl-ignore-chunk` comments that use a single-line form
/// instead of the required Quarto YAML array form.
///
/// ## Why is this bad?
///
/// In Quarto and R Markdown documents, `#|` comments are parsed as YAML chunk
/// options. The single-line form
///
/// ```r
/// #| jarl-ignore-chunk any_is_na: <reason>
/// ```
///
/// is not idiomatic YAML and therefore Quarto will not compile. The correct
/// form is a YAML array:
///
/// ```r
/// #| jarl-ignore-chunk:
/// #|   - any_is_na: <reason>
/// ```
///
/// ## Example
///
/// ```r
/// #| jarl-ignore-chunk any_is_na: <reason>
/// any(is.na(x))
/// ```
///
/// Use instead:
///
/// ```r
/// #| jarl-ignore-chunk:
/// #|   - any_is_na: <reason>
/// any(is.na(x))
/// ```
pub fn invalid_chunk_suppression(ranges: &[TextRange]) -> Vec<Diagnostic> {
    ranges
        .iter()
        .map(|range| create_diagnostic(*range))
        .collect()
}

fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "invalid_chunk_suppression".to_string(),
            "This `jarl-ignore-chunk` comment is wrongly formatted.".to_string(),
            Some(
                "Use the YAML array form instead:\n\
                 #| jarl-ignore-chunk:\n\
                 #|   - <rule>: <reason>"
                    .to_string(),
            ),
        ),
        range,
        Fix::empty(),
    )
}
