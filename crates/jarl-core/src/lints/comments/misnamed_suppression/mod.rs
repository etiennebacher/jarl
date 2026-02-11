pub(crate) mod misnamed_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "misnamed_suppression", None)
    }

    #[test]
    fn test_no_lint_misnamed_suppression() {
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
any(is.na(x))",
            "misnamed_suppression",
            None,
        );

        expect_no_lint(
            "
# jarl-ignore-file any_is_na: <reason>
any(is.na(x))",
            "misnamed_suppression",
            None,
        );

        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na",
            "misnamed_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misnamed_suppression() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore any_isna: <reason>
any(is.na(x))"), @r"
        warning: misnamed_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_isna: <reason>
          | -------------------------------- This comment isn't used by Jarl because it contains an unrecognized rule name.
          |
          = help: Check the rule name for typos.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_misnamed_suppression_file() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-file nonexistent_rule: <reason>
any(is.na(x))"), @r"
        warning: misnamed_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-file nonexistent_rule: <reason>
          | --------------------------------------------- This comment isn't used by Jarl because it contains an unrecognized rule name.
          |
          = help: Check the rule name for typos.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_misnamed_suppression_region_start() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start fake_rule: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na"), @r"
        warning: misnamed_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start fake_rule: <reason>
          | --------------------------------------- This comment isn't used by Jarl because it contains an unrecognized rule name.
          |
          = help: Check the rule name for typos.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_misnamed_suppression_region_end() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end fake_rule"), @r"
        warning: misnamed_suppression
         --> <test>:4:1
          |
        4 | # jarl-ignore-end fake_rule
          | --------------------------- This comment isn't used by Jarl because it contains an unrecognized rule name.
          |
          = help: Check the rule name for typos.
        Found 1 error.
        "
        );
    }
}
