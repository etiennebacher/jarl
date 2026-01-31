pub(crate) mod misplaced_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_misplaced_suppression() {
        // Suppression on its own line is valid
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
any(is.na(x))",
            "misplaced_suppression",
            None,
        );

        // File suppression at top is valid
        expect_no_lint(
            "
# jarl-ignore-file any_is_na: <reason>
any(is.na(x))",
            "misplaced_suppression",
            None,
        );

        // Region suppression is valid
        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na
x <- 1",
            "misplaced_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misplaced_suppression() {
        let lint_msg = "end-of-line suppressions are not supported";

        // Trailing suppression comment
        expect_lint(
            "any(is.na(x)) # jarl-ignore any_is_na: <reason>",
            lint_msg,
            "misplaced_suppression",
            None,
        );

        // Trailing suppression comment
        expect_lint(
            "
any(
  is.na(x)
) # jarl-ignore any_is_na: <reason>",
            lint_msg,
            "misplaced_suppression",
            None,
        );

        // Trailing file suppression
        expect_lint(
            "any(is.na(x)) # jarl-ignore-file any_is_na: <reason>",
            lint_msg,
            "misplaced_suppression",
            None,
        );

        // Trailing region start
        expect_lint(
            "any(is.na(x)) # jarl-ignore-start any_is_na: <reason>",
            lint_msg,
            "misplaced_suppression",
            None,
        );
    }
}
