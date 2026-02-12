pub(crate) mod duplicated_arguments;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "duplicated_arguments", None)
    }

    #[test]
    fn test_no_lint_duplicated_arguments() {
        expect_no_lint("fun(arg = 1)", "duplicated_arguments", None);
        expect_no_lint("fun('arg' = 1)", "duplicated_arguments", None);
        expect_no_lint("fun(`arg` = 1)", "duplicated_arguments", None);
        expect_no_lint("'fun'(arg = 1)", "duplicated_arguments", None);
        expect_no_lint(
            "(function(x, y) x + y)(x = 1)",
            "duplicated_arguments",
            None,
        );
        expect_no_lint(
            "fun(x = (function(x) x + 1), y = 1)",
            "duplicated_arguments",
            None,
        );
        expect_no_lint("dt[i = 1]", "duplicated_arguments", None);
        expect_no_lint(
            "cli_format_each_inline(x = 'a', x = 'a')",
            "duplicated_arguments",
            None,
        );

        // `"` and `'` are not the same argument names.
        expect_no_lint("switch(x, `\"` = 1, `'` = 2)", "duplicated_arguments", None);
    }

    #[test]
    fn test_lint_duplicated_arguments() {
        assert_snapshot!(
            snapshot_lint("fun(arg = 1, arg = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | fun(arg = 1, arg = 2)
          | --------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("fun(arg = 1, 'arg' = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | fun(arg = 1, 'arg' = 2)
          | ----------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("fun(arg = 1, `arg` = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | fun(arg = 1, `arg` = 2)
          | ----------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("'fun'(arg = 1, arg = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | 'fun'(arg = 1, arg = 2)
          | ----------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("list(a = 1, a = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | list(a = 1, a = 2)
          | ------------------ Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("foo(a = 1, a = function(x) 1)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | foo(a = 1, a = function(x) 1)
          | ----------------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("foo(a = 1, a = (function(x) x + 1))"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | foo(a = 1, a = (function(x) x + 1))
          | ----------------------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
        // TODO
        // assert!(expect_lint(
        //     "dt[i = 1, i = 2]",
        //     expected_message,
        //     "duplicated_arguments"
        // ));
    }

    #[test]
    fn test_duplicated_arguments_accepted_functions() {
        expect_no_lint(
            "dplyr::mutate(x, a = 1, a = 2)",
            "duplicated_arguments",
            None,
        );
        expect_no_lint("transmute(x, a = 1, a = 2)", "duplicated_arguments", None);
    }

    #[test]
    fn test_duplicated_arguments_no_nested_functions() {
        expect_no_lint(
            "foo(x = {
            bar(a = 1)
            baz(a = 1)
        })",
            "duplicated_arguments",
            None,
        );
    }

    #[test]
    fn test_duplicated_arguments_no_args() {
        expect_no_lint("foo()", "duplicated_arguments", None);
    }

    #[test]
    fn test_duplicated_arguments_with_interceding_comments() {
        assert_snapshot!(
            snapshot_lint(
            "fun(
                arg # xxx
                = 1,
                arg # yyy
                = 2
              )"), @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | / fun(
        2 | |                 arg # xxx
        ... |
        5 | |                 = 2
        6 | |               )
          | |_______________- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint(
            "fun(
                arg = # xxx
                1,
                arg = # yyy
                2
              )"), @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | / fun(
        2 | |                 arg = # xxx
        ... |
        5 | |                 2
        6 | |               )
          | |_______________- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
    }
}
