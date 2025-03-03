pub(crate) mod any_is_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_any_na() {
        use insta::assert_snapshot;
        let (lint_output, fix_output) = get_lint_and_fix_text(
            vec![
                "any(is.na(x))",
                "any(is.na(foo(x)))",
                "any(is.na(x), na.rm = TRUE)",
            ],
            "any_is_na",
        );
        assert_snapshot!("lint_output", lint_output);
        assert_snapshot!("fix_output", fix_output);
    }

    #[test]
    fn test_no_lint_any_na() {
        assert!(no_lint("y <- any(x)", "any_is_na"));
        assert!(no_lint("y <- is.na(x)", "any_is_na"));
        assert!(no_lint("y <- any(!is.na(x))", "any_is_na"));
        assert!(no_lint("y <- any(!is.na(foo(x)))", "any_is_na"))
    }
}
