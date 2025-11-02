pub(crate) mod class_equals;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_class_equals() {
        use insta::assert_snapshot;

        let expected_message = "Comparing `class(x)` with";

        expect_lint(
            "if (class(x) == 'character') 1",
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
            "while (class(x) != 'character') 1",
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

        // No fixes because we can't infer if it is correct or not.
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
            get_fixed_text(
                vec![
                    "is_regression <- class(x) == 'lm'",
                    "if (class(x) == 'character') 1",
                    "is_regression <- 'lm' == class(x)",
                    "is_regression <- \"lm\" == class(x)",
                    "if ('character' %in% class(x)) 1",
                    "if (class(x) %in% 'character') 1",
                    "if (class(x) != 'character') 1",
                    "while (class(x) != 'character') 1",
                    "x[if (class(x) == 'foo') 1 else 2]",
                    "if (class(foo(bar(y) + 1)) == 'abc') 1",
                    "if (my_fun(class(x) != 'character')) 1",
                ],
                "class_equals",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_class_equals() {
        expect_no_lint("class(x) <- 'character'", "class_equals", None);
        expect_no_lint(
            "identical(class(x), c('glue', 'character'))",
            "class_equals",
            None,
        );
        expect_no_lint("all(sup %in% class(model))", "class_equals", None);

        // We cannot infer the use that will be made of this output, so we can't
        // report it:
        expect_no_lint("is_regression <- class(x) == 'lm'", "class_equals", None);
        expect_no_lint("is_regression <- 'lm' == class(x)", "class_equals", None);
        expect_no_lint("is_regression <- \"lm\" == class(x)", "class_equals", None);
    }

    #[test]
    fn test_class_equals_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nif (class(x) == 'foo') 1",
                    "if(\n  class(\n  # comment\nx) == 'foo'\n) 1",
                    "if (class(x) == 'foo') 1 # trailing comment",
                ],
                "class_equals",
                None
            )
        );
    }
}
