pub(crate) mod misplaced_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "misplaced_suppression", None)
    }

    #[test]
    fn test_no_lint_misplaced_suppression() {
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
any(is.na(x))",
            "misplaced_suppression",
            None,
        );

        expect_no_lint(
            "
# jarl-ignore-file any_is_na: <reason>
any(is.na(x))",
            "misplaced_suppression",
            None,
        );

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
        insta::assert_snapshot!(
            snapshot_lint(
            "any(is.na(x)) # jarl-ignore any_is_na: <reason>"), @r"
        warning: misplaced_suppression
         --> <test>:1:15
          |
        1 | any(is.na(x)) # jarl-ignore any_is_na: <reason>
          |               --------------------------------- This comment isn't used by Jarl because end-of-line suppressions are not supported.
          |
          = help: Move the suppression comment to its own line above the code you want to suppress.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
any(
  is.na(x)
) # jarl-ignore any_is_na: <reason>"), @r"
        warning: misplaced_suppression
         --> <test>:4:3
          |
        4 | ) # jarl-ignore any_is_na: <reason>
          |   --------------------------------- This comment isn't used by Jarl because end-of-line suppressions are not supported.
          |
          = help: Move the suppression comment to its own line above the code you want to suppress.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(

            snapshot_lint(
            "any(is.na(x)) # jarl-ignore-file any_is_na: <reason>"), @r"
        warning: misplaced_suppression
         --> <test>:1:15
          |
        1 | any(is.na(x)) # jarl-ignore-file any_is_na: <reason>
          |               -------------------------------------- This comment isn't used by Jarl because end-of-line suppressions are not supported.
          |
          = help: Move the suppression comment to its own line above the code you want to suppress.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(

            snapshot_lint(
            "any(is.na(x)) # jarl-ignore-start any_is_na: <reason>"), @r"
        warning: misplaced_suppression
         --> <test>:1:15
          |
        1 | any(is.na(x)) # jarl-ignore-start any_is_na: <reason>
          |               --------------------------------------- This comment isn't used by Jarl because end-of-line suppressions are not supported.
          |
          = help: Move the suppression comment to its own line above the code you want to suppress.
        Found 1 error.
        "
        );
    }
}
