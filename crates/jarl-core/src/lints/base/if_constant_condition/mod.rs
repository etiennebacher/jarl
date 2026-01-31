pub(crate) mod if_constant_condition;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_if_constant_condition() {
        expect_no_lint("if (x) { print('hi') }", "if_constant_condition", None);

        // This is handled by `unreachable_code`
        expect_no_lint(
            "if (TRUE) { print('hi') } else { print('bye') }",
            "if_constant_condition",
            None,
        );

        // This is handled by `unreachable_code`
        expect_no_lint(
            "if (FALSE) { print('hi') } else { print('bye') }",
            "if_constant_condition",
            None,
        );

        expect_no_lint(
            "if (TRUE && x) { print('hi') }",
            "if_constant_condition",
            None,
        );
        expect_no_lint(
            "if (x || FALSE) { print('hi') }",
            "if_constant_condition",
            None,
        );
        //Handled by `unreachable_code`
        expect_no_lint(
            "if (x) { print('hi') } else if (TRUE) { print('bye') } else { print('unreachable') }",
            "if_constant_condition",
            None,
        );
    }

    #[test]
    fn test_lint_if_constant_condition() {
        expect_lint(
            "if (TRUE) print('hi')",
            "always `TRUE`",
            "if_constant_condition",
            None,
        );
        expect_lint(
            "if (FALSE) { print('hi') }",
            "always `FALSE`",
            "if_constant_condition",
            None,
        );
        expect_lint(
            "if (TRUE || x) { print('hi') }",
            "always `TRUE`",
            "if_constant_condition",
            None,
        );
        expect_lint(
            "if (FALSE && x) { print('hi') }",
            "always `FALSE`",
            "if_constant_condition",
            None,
        );
        expect_lint(
            "if (x || TRUE) { print('hi') }",
            "always `TRUE`",
            "if_constant_condition",
            None,
        );
        expect_lint(
            "if (x && FALSE) { print('hi') }",
            "always `FALSE`",
            "if_constant_condition",
            None,
        );
        expect_lint(
            "if (!FALSE) { print('hi') }",
            "always `TRUE`",
            "if_constant_condition",
            None,
        );
        //Not handled by `unreachable_code`
        expect_lint(
            "if (x) { print('hi') } else if (TRUE) { print('bye') }",
            "always `TRUE`",
            "if_constant_condition",
            None,
        );
    }
}
