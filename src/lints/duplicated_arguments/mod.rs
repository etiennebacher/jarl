pub(crate) mod duplicated_arguments;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_duplicated_arguments() {
        expect_no_lint("fun(arg = 1)", "duplicated_arguments");
        expect_no_lint("fun('arg' = 1)", "duplicated_arguments");
        expect_no_lint("fun(`arg` = 1)", "duplicated_arguments");
        expect_no_lint("'fun'(arg = 1)", "duplicated_arguments");
        expect_no_lint("(function(x, y) x + y)(x = 1)", "duplicated_arguments");
        expect_no_lint(
            "fun(x = (function(x) x + 1), y = 1)",
            "duplicated_arguments",
        );
        expect_no_lint("dt[i = 1]", "duplicated_arguments");
    }

    #[test]
    fn test_lint_duplicated_arguments() {
        let expected_message = "Avoid duplicate arguments in function";
        expect_lint(
            "fun(arg = 1, arg = 2)",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "fun(arg = 1, 'arg' = 2)",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "fun(arg = 1, `arg` = 2)",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "'fun'(arg = 1, arg = 2)",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "list(a = 1, a = 2)",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "foo(a = 1, a = function(x) 1)",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "foo(a = 1, a = (function(x) x + 1))",
            expected_message,
            "duplicated_arguments",
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
        expect_no_lint("dplyr::mutate(x, a = 1, a = 2)", "duplicated_arguments");
        expect_no_lint("transmute(x, a = 1, a = 2)", "duplicated_arguments");
    }

    #[test]
    fn test_duplicated_arguments_no_nested_functions() {
        expect_no_lint(
            "foo(x = {
            bar(a = 1)
            baz(a = 1)
        })",
            "duplicated_arguments",
        );
    }

    #[test]
    fn test_duplicated_arguments_no_args() {
        expect_no_lint("foo()", "duplicated_arguments");
    }

    #[test]
    fn test_duplicated_arguments_with_interceding_comments() {
        let expected_message = "Avoid duplicate arguments in function";

        expect_lint(
            "fun(
                arg # xxx
                = 1,
                arg # yyy
                = 2
              )",
            expected_message,
            "duplicated_arguments",
        );
        expect_lint(
            "fun(
                arg = # xxx
                1,
                arg = # yyy
                2
              )",
            expected_message,
            "duplicated_arguments",
        );
    }
}
