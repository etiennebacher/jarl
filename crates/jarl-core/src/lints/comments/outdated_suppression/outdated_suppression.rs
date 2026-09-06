use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::suppression::UnusedSuppression;
use crate::utils::{line_start, next_line_start};

/// Version added: 0.4.0
///
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
/// This rule has a safe automatic fix that removes the outdated comment.
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
pub fn outdated_suppression(unused: &[UnusedSuppression], source: &str) -> Vec<Diagnostic> {
    unused
        .iter()
        .map(|suppression| create_diagnostic(suppression, source))
        .collect()
}

fn create_diagnostic(suppression: &UnusedSuppression, source: &str) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            Rule::OutdatedSuppression,
            "This suppression comment is unused, no violation would be reported without it."
                .to_string(),
            Some("Remove this suppression comment or verify that it's still needed.".to_string()),
        ),
        suppression.comment_range,
        create_fix(suppression, source),
    )
}

/// Remove the suppression comment, along with the line it sits on.
///
/// A suppression comment is always alone on its line (a trailing one is
/// reported by `misplaced_suppression` instead and never suppresses anything),
/// so the indentation and the line break go with it.
///
/// A `jarl-ignore-start`/`jarl-ignore-end` pair needs both comments gone, which
/// is one deletion per comment; the code they wrap is untouched.
fn create_fix(suppression: &UnusedSuppression, source: &str) -> Fix {
    let comment = suppression.comment_range;
    let mut edits = vec![delete_line(source, comment.start().into())];

    // For a region, the closing comment sits on its own line further down.
    if let Some(region) = suppression.region_range {
        edits.push(delete_line(source, region.end().into()));
    }

    Fix::from_edits(edits, false)
}

/// Delete the whole line containing `offset`, line break included.
fn delete_line(source: &str, offset: usize) -> Edit {
    Edit::deletion_with_offsets(line_start(source, offset), next_line_start(source, offset))
}
