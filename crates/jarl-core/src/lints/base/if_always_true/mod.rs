pub(crate) mod if_always_true;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_if_always_true() {
        expect_no_lint("if (x) { print('hi') }", "if_always_true", None);

        // This is handled by `unreachable_code`
        expect_no_lint(
            "if (TRUE) { print('hi') } else { print('bye') }",
            "if_always_true",
            None,
        );

        // This is handled by `unreachable_code`
        expect_no_lint(
            "if (!FALSE) { print('hi') } else { print('bye') }",
            "if_always_true",
            None,
        );

        expect_no_lint("if (TRUE && x) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (x || FALSE) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (0 || x) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (0) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (-0.0) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (x && FALSE) { print('hi') }", "if_always_true", None);
        //Handled by `unreachable_code`
        expect_no_lint(
            "if (x) { print('hi') } else if (TRUE) { print('bye') } else { print('unreachable') }",
            "if_always_true",
            None,
        );
    }

    #[test]
    fn test_lint_if_always_true() {
        let message = "always evaluates to `TRUE`";
        expect_lint("if (TRUE) print('hi')", message, "if_always_true", None);
        expect_lint(
            "if (TRUE || x) { print('hi') }",
            message,
            "if_always_true",
            None,
        );
        expect_lint(
            "if (x || TRUE) { print('hi') }",
            message,
            "if_always_true",
            None,
        );
        expect_lint("if (1) { print('hi') }", message, "if_always_true", None);
        expect_lint("if (-1) { print('hi') }", message, "if_always_true", None);
        expect_lint("if (5.5) { print('hi') }", message, "if_always_true", None);
        expect_lint("if (0.1) { print('hi') }", message, "if_always_true", None);
        expect_lint(
            "if (10 || x) { print('hi') }",
            message,
            "if_always_true",
            None,
        );
        expect_lint(
            "if (!FALSE) { print('hi') }",
            message,
            "if_always_true",
            None,
        );
        expect_lint("if (Inf) { print('hi') }", message, "if_always_true", None);
        expect_lint("if (-Inf) { print('hi') }", message, "if_always_true", None);
        //Not handled by `unreachable_code`
        expect_lint(
            "if (x) { print('hi') } else if (TRUE) { print('bye') }",
            message,
            "if_always_true",
            None,
        );
    }
}
