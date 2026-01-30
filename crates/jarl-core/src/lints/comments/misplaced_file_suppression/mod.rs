pub(crate) mod misplaced_file_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_misplaced_file_suppression() {
        // File suppression at the top of the file is valid
        expect_no_lint(
            "
# jarl-ignore-file any_is_na: this is needed
any(is.na(x))",
            "misplaced_file_suppression",
            None,
        );

        // Multiple file suppressions at top are valid
        expect_no_lint(
            "
# jarl-ignore-file any_is_na: reason 1
# jarl-ignore-file browser: reason 2
any(is.na(x))",
            "misplaced_file_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misplaced_file_suppression() {
        let lint_msg = "must be at the top of the file";

        // File suppression after code
        expect_lint(
            "
x <- 1
# jarl-ignore-file any_is_na: explanation
any(is.na(x))",
            lint_msg,
            "misplaced_file_suppression",
            None,
        );

        // Some file suppressions are misplaced
        expect_lint(
            "
# jarl-ignore-file any_is_na: reason 1
x <- 1
# jarl-ignore-file browser: reason 2
any(is.na(x))",
            lint_msg,
            "misplaced_file_suppression",
            None,
        );
    }
}
