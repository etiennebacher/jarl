pub(crate) mod true_false_symbol;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_any_duplicated() {
        use insta::assert_snapshot;
        let (lint_output, fix_output) = get_lint_and_fix_text(
            vec![
                "any(duplicated(x))",
                "any(duplicated(foo(x)))",
                "any(duplicated(x), na.rm = TRUE)",
            ],
            "any_duplicated",
        );
        assert_snapshot!("lint_output", lint_output);
        assert_snapshot!("fix_output", fix_output);
    }

    #[test]
    fn test_no_lint_any_duplicated() {
        assert!(no_lint("y <- any(x)", "any_duplicated",));
        assert!(no_lint("y <- duplicated(x)", "any_duplicated",));
        assert!(no_lint("y <- any(!duplicated(x))", "any_duplicated",));
        assert!(no_lint("y <- any(!duplicated(foo(x)))", "any_duplicated",))
    }
}
