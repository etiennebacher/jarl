use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::suppression::UnusedSuppression;

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
/// A `jarl-ignore-start`/`jarl-ignore-end` pair needs both comments gone, but a
/// fix is a single contiguous replacement: the edit therefore spans the whole
/// region and puts back the code it wraps.
fn create_fix(suppression: &UnusedSuppression, source: &str) -> Fix {
    let comment = suppression.comment_range;
    let last_comment_end = suppression.region_range.unwrap_or(comment).end().into();

    // For a region, the code between the two comments is put back untouched.
    let content = match suppression.region_range {
        Some(_) => source
            [next_line_start(source, comment.end().into())..line_start(source, last_comment_end)]
            .to_string(),
        None => String::new(),
    };

    Fix {
        content,
        start: line_start(source, comment.start().into()),
        end: next_line_start(source, last_comment_end),
        to_skip: false,
    }
}

/// Offset of the first character of the line containing `offset`.
fn line_start(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |i| i + 1)
}

/// Offset just past the line break ending the line that contains `offset`, or
/// the end of the source if that line is the last one.
fn next_line_start(source: &str, offset: usize) -> usize {
    source[offset..]
        .find('\n')
        .map_or(source.len(), |i| offset + i + 1)
}
