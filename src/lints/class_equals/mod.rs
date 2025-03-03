pub(crate) mod class_equals;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_class_equals() {
        use insta::assert_snapshot;
        let (lint_output, fix_output) = get_lint_and_fix_text(
            vec![
                "is_regression <- class(x) == 'lm'",
                "if (class(x) == 'character') 1",
                "is_regression <- 'lm' == class(x)",
                "is_regression <- \"lm\" == class(x)",
                // TODO: those two should fix
                "if ('character' %in% class(x)) 1",
                "if (class(x) %in% 'character') 1",
                "if (class(x) != 'character') 1",
                "x[if (class(x) == 'foo') 1 else 2]",
                "class(foo(bar(y) + 1)) == 'abc'",
            ],
            "class_equals",
        );
        assert_snapshot!("lint_output", lint_output);
        assert_snapshot!("fix_output", fix_output);
    }

    #[test]
    fn test_no_lint_class_equals() {
        assert!(no_lint("class(x) <- 'character'", "class_equals"));
        assert!(no_lint("class(x) = 'character'", "class_equals"));
        assert!(no_lint(
            "identical(class(x), c('glue', 'character'))",
            "class_equals"
        ));
        assert!(no_lint("all(sup %in% class(model))", "class_equals"));
        assert!(no_lint("class(x)[class(x) == 'foo']", "class_equals"));
    }
}
