pub(crate) mod misnamed_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_misnamed_suppression() {
        // Valid rule name
        expect_no_lint(
            "
# jarl-ignore any_is_na: <explanation>
any(is.na(x))",
            "misnamed_suppression",
            None,
        );

        // Valid file suppression
        expect_no_lint(
            "
# jarl-ignore-file any_is_na: <explanation>
any(is.na(x))",
            "misnamed_suppression",
            None,
        );

        // Valid region suppression
        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <explanation>
any(is.na(x))
# jarl-ignore-end any_is_na",
            "misnamed_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misnamed_suppression() {
        let lint_msg = "unrecognized rule name";

        // Typo in rule name
        expect_lint(
            "
# jarl-ignore any_isna: <explanation>
any(is.na(x))",
            lint_msg,
            "misnamed_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misnamed_suppression_file() {
        let lint_msg = "unrecognized rule name";

        // Non-existent rule in file suppression
        expect_lint(
            "
# jarl-ignore-file nonexistent_rule: <explanation>
any(is.na(x))",
            lint_msg,
            "misnamed_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misnamed_suppression_region_start() {
        let lint_msg = "unrecognized rule name";

        // Non-existent rule in region start
        expect_lint(
            "
# jarl-ignore-start fake_rule: <explanation>
any(is.na(x))
# jarl-ignore-end any_is_na",
            lint_msg,
            "misnamed_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_misnamed_suppression_region_end() {
        let lint_msg = "unrecognized rule name";

        // Non-existent rule in region end
        expect_lint(
            "
# jarl-ignore-start any_is_na: <explanation>
any(is.na(x))
# jarl-ignore-end fake_rule",
            lint_msg,
            "misnamed_suppression",
            None,
        );
    }
}
