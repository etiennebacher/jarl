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
            None,
        );
        expect_lint(
            "if (class(x) == 'character') 1",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "is_regression <- 'lm' == class(x)",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "is_regression <- \"lm\" == class(x)",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "if ('character' %in% class(x)) 1",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "if (class(x) %in% 'character') 1",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "if (class(x) != 'character') 1",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "x[if (class(x) == 'foo') 1 else 2]",
            expected_message,
            "class_equals",
            None,
        );
        expect_lint(
            "class(foo(bar(y) + 1)) == 'abc'",
            expected_message,
            "class_equals",
            None,
        );

        // No fixes because it is unsafe
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(
                vec!["is_regression <- class(x) == 'lm'",],
                "class_equals",
                None
            )
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
        expect_no_lint("class(x) <- 'character'", "class_equals", None);
        expect_no_lint("class(x) = 'character'", "class_equals", None);
        expect_no_lint(
            "identical(class(x), c('glue', 'character'))",
            "class_equals",
            None,
        );
        expect_no_lint("all(sup %in% class(model))", "class_equals", None);

        // TODO: https://github.com/etiennebacher/flir2/issues/32
        // expect_no_lint("class(x)[class(x) == 'foo']", "class_equals");
    }

    #[test]
    fn test_class_equals_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nclass(x) == 'lm'",
                    "class(\n  # comment\n  x\n) == 'lm'",
                    "# comment\nclass(x) == 'character'",
                    "class(x) == 'lm' # trailing comment",
                ],
                "class_equals"
            )
        );
    }
}
