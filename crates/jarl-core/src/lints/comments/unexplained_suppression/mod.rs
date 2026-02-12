pub(crate) mod unexplained_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unexplained_suppression", None)
    }

    #[test]
    fn test_no_lint_unexplained_suppression() {
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
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore any_is_na
any(is.na(x))"), @r"
        warning: unexplained_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_is_na
          | ----------------------- This comment isn't used by Jarl because it is missing an explanation.
          |
          = help: Add an explanation after the colon, e.g., `# jarl-ignore rule: <reason>`.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore any_is_na:
any(is.na(x))"), @r"
        warning: unexplained_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_is_na:
          | ------------------------ This comment isn't used by Jarl because it is missing an explanation.
          |
          = help: Add an explanation after the colon, e.g., `# jarl-ignore rule: <reason>`.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(

            snapshot_lint(
            "\n# jarl-ignore any_is_na:     \nany(is.na(x))"), @r"
        warning: unexplained_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_is_na:     
          | ----------------------------- This comment isn't used by Jarl because it is missing an explanation.
          |
          = help: Add an explanation after the colon, e.g., `# jarl-ignore rule: <reason>`.
        Found 1 error.
        "
        );
    }
}
