pub(crate) mod class_equals;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_class_equals() {
        use insta::assert_snapshot;

        let expected_message = "instead of comparing `class";

        expect_lint(
            "is_regression <- class(x) == 'lm'",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "if (class(x) == 'character') 1",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "is_regression <- 'lm' == class(x)",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "is_regression <- \"lm\" == class(x)",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "if ('character' %in% class(x)) 1",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "if (class(x) %in% 'character') 1",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "if (class(x) != 'character') 1",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "x[if (class(x) == 'foo') 1 else 2]",
            expected_message,
            "class_equals",
        );
        expect_lint(
            "class(foo(bar(y) + 1)) == 'abc'",
            expected_message,
            "class_equals",
        );

        // No fixes because it is unsafe
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(vec!["is_regression <- class(x) == 'lm'",], "class_equals")
        );

        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "is_regression <- class(x) == 'lm'",
                    "if (class(x) == 'character') 1",
                    "is_regression <- 'lm' == class(x)",
                    "is_regression <- \"lm\" == class(x)",
                    "if ('character' %in% class(x)) 1",
                    "if (class(x) %in% 'character') 1",
                    "if (class(x) != 'character') 1",
                    "x[if (class(x) == 'foo') 1 else 2]",
                    "class(foo(bar(y) + 1)) == 'abc'",
                ],
                "class_equals"
            )
        );
    }

    #[test]
    fn test_no_lint_class_equals() {
        expect_no_lint("class(x) <- 'character'", "class_equals");
        expect_no_lint("class(x) = 'character'", "class_equals");
        expect_no_lint(
            "identical(class(x), c('glue', 'character'))",
            "class_equals",
        );
        expect_no_lint("all(sup %in% class(model))", "class_equals");

        // TODO: https://github.com/etiennebacher/flir2/issues/32
        // expect_no_lint("class(x)[class(x) == 'foo']", "class_equals");
    }
}
