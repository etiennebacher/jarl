pub(crate) mod class_equals;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "class_equals", None)
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

        expect_no_lint("identical(foo(x), 'a')", "class_equals", None);
        expect_no_lint("identical(foo(x), c('a', 'b'))", "class_equals", None);
    }

    #[test]
    fn test_lint_class_equals() {
        assert_snapshot!(
            snapshot_lint("if (class(x) == 'character') 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if (class(x) == 'character') 1
          |     ----------------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (base::class(x) == 'character') 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if (base::class(x) == 'character') 1
          |     ----------------------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if ('character' %in% class(x)) 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if ('character' %in% class(x)) 1
          |     ------------------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (class(x) %in% 'character') 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if (class(x) %in% 'character') 1
          |     ------------------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (class(x) != 'character') 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if (class(x) != 'character') 1
          |     ----------------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (class(x) != 'character') 1"),
            @r"
        warning: class_equals
         --> <test>:1:8
          |
        1 | while (class(x) != 'character') 1
          |        ----------------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[if (class(x) == 'foo') 1 else 2]"),
            @r"
        warning: class_equals
         --> <test>:1:7
          |
        1 | x[if (class(x) == 'foo') 1 else 2]
          |       ----------------- Comparing `class(x)` with `==` or `%in%` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
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
    fn test_lint_identical_class() {
        assert_snapshot!(
            snapshot_lint("is_regression <- identical(class(x), 'lm')"),
            @r"
        warning: class_equals
         --> <test>:1:18
          |
        1 | is_regression <- identical(class(x), 'lm')
          |                  ------------------------- Using `identical(class(x), 'a')` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("is_regression <- identical('lm', class(x))"),
            @r"
        warning: class_equals
         --> <test>:1:18
          |
        1 | is_regression <- identical('lm', class(x))
          |                  ------------------------- Using `identical(class(x), 'a')` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (identical(class(x), 'character')) 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if (identical(class(x), 'character')) 1
          |     -------------------------------- Using `identical(class(x), 'a')` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (identical('character', class(x))) 1"),
            @r"
        warning: class_equals
         --> <test>:1:5
          |
        1 | if (identical('character', class(x))) 1
          |     -------------------------------- Using `identical(class(x), 'a')` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (identical(class(x), 'foo')) 1"),
            @r"
        warning: class_equals
         --> <test>:1:8
          |
        1 | while (identical(class(x), 'foo')) 1
          |        -------------------------- Using `identical(class(x), 'a')` can be problematic.
          |
          = help: Use `inherits(x, 'a')` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "identical_class",
            get_fixed_text(
                vec![
                    "if (identical(class(x), 'character')) 1",
                    "if (identical('character', class(x))) 1",
                    "while (identical(class(x), 'foo')) 1",
                    "is_regression <- identical(class(x), 'lm')",
                    "is_regression <- identical('lm', class(x))",
                ],
                "class_equals",
                None
            )
        );
    }

    #[test]
    fn test_class_equals_with_comments_no_fix() {
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
