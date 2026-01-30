pub(crate) mod unexplained_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_unexplained_suppression() {
        // Valid suppression with explanation
        expect_no_lint(
            "
# jarl-ignore any_is_na: this is needed for performance
any(is.na(x))",
            "unexplained_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_unexplained_suppression() {
        let lint_msg = "is missing an explanation";

        // No colon at all
        expect_lint(
            "
# jarl-ignore any_is_na
any(is.na(x))",
            lint_msg,
            "unexplained_suppression",
            None,
        );

        // Colon but empty explanation
        expect_lint(
            "
# jarl-ignore any_is_na:
any(is.na(x))",
            lint_msg,
            "unexplained_suppression",
            None,
        );

        // Colon with only whitespace
        expect_lint(
            "
# jarl-ignore any_is_na:     \
any(is.na(x))",
            lint_msg,
            "unexplained_suppression",
            None,
        );
    }
}
