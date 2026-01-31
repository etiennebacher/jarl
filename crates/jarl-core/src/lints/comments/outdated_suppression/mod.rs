pub(crate) mod outdated_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    // Note: For outdated_suppression tests, we need to enable both the
    // outdated_suppression rule AND the rule that would be suppressed.
    // Otherwise the suppression will always appear unused because
    // the violation wouldn't be reported anyway.

    #[test]
    fn test_no_lint_outdated_suppression() {
        // Suppression that actually suppresses a violation is not outdated
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
any(is.na(x))",
            "outdated_suppression,any_is_na",
            None,
        );
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
f <- function(x) {
  any(is.na(x))
}",
            "outdated_suppression,any_is_na",
            None,
        );

        // File-level suppression that suppresses a violation
        expect_no_lint(
            "
# jarl-ignore-file any_is_na: <reason>
any(is.na(x))
any(is.na(y))",
            "outdated_suppression,any_is_na",
            None,
        );

        // Region suppression that suppresses a violation
        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na
x <- 1",
            "outdated_suppression,any_is_na",
            None,
        );
    }

    #[test]
    fn test_lint_outdated_suppression() {
        let lint_msg = "This suppression comment is unused";

        // Suppression with no violation to suppress
        expect_lint(
            "
# jarl-ignore any_is_na: <reason>
x <- 1",
            lint_msg,
            "outdated_suppression,any_is_na",
            None,
        );
        expect_lint(
            "
# jarl-ignore any_is_na: <reason>
f <- function(x) {
  1 + 1
}",
            lint_msg,
            "outdated_suppression,any_is_na",
            None,
        );

        // File-level suppression with no violation to suppress
        expect_lint(
            "
# jarl-ignore-file any_is_na: <reason>
x <- 1
y <- 2",
            lint_msg,
            "outdated_suppression,any_is_na",
            None,
        );

        // Region suppression with no violation to suppress
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
x <- 1
# jarl-ignore-end any_is_na
y <- 2",
            lint_msg,
            "outdated_suppression,any_is_na",
            None,
        );
    }

    #[test]
    fn test_lint_outdated_suppression_wrong_rule() {
        let lint_msg = "This suppression comment is unused";

        // Suppression for wrong rule (any_is_na suppression, but violation is equals_na)
        expect_lint(
            "
# jarl-ignore any_is_na: <reason>
x == NA",
            lint_msg,
            "outdated_suppression,any_is_na,equals_na",
            None,
        );
    }
}
