pub(crate) mod misplaced_file_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "misplaced_file_suppression", None)
    }

    #[test]
    fn test_no_lint_misplaced_file_suppression() {
        expect_no_lint(
            "
# jarl-ignore-file any_is_na: this is needed
any(is.na(x))",
            "misplaced_file_suppression",
            None,
        );

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
        insta::assert_snapshot!(snapshot_lint("
x <- 1
# jarl-ignore-file any_is_na: explanation
any(is.na(x))"), @r"
        warning: misplaced_file_suppression
         --> <test>:3:1
          |
        3 | # jarl-ignore-file any_is_na: explanation
          | ----------------------------------------- This comment isn't used by Jarl because `# jarl-ignore-file` must be at the top of the file.
          |
          = help: Move this comment to the beginning of the file, before any code.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-file any_is_na: reason 1
x <- 1
# jarl-ignore-file browser: reason 2
any(is.na(x))"), @r"
        warning: misplaced_file_suppression
         --> <test>:4:1
          |
        4 | # jarl-ignore-file browser: reason 2
          | ------------------------------------ This comment isn't used by Jarl because `# jarl-ignore-file` must be at the top of the file.
          |
          = help: Move this comment to the beginning of the file, before any code.
        Found 1 error.
        "
        );
    }
}
