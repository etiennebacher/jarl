pub(crate) mod equals_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_equals_na() {
        use insta::assert_snapshot;
        let (lint_output, fix_output) = get_lint_and_fix_text(
            vec![
                "x == NA",
                "x == NA_integer_",
                "x == NA_real_",
                "x == NA_logical_",
                "x == NA_character_",
                "x == NA_complex_",
                "x != NA",
                "foo(x(y)) == NA",
                "NA == x",
            ],
            "equals_na",
        );
        assert_snapshot!("lint_output", lint_output);
        assert_snapshot!("fix_output", fix_output);
    }

    #[test]
    fn test_no_lint_equals_na() {
        assert!(no_lint("x + NA", "equals_na"));
        assert!(no_lint("x == \"NA\"", "equals_na"));
        assert!(no_lint("x == 'NA'", "equals_na"));
        assert!(no_lint("x <- NA", "equals_na"));
        assert!(no_lint("x <- NaN", "equals_na"));
        assert!(no_lint("x <- NA_real_", "equals_na"));
        assert!(no_lint("is.na(x)", "equals_na"));
        assert!(no_lint("is.nan(x)", "equals_na"));
        assert!(no_lint("x[!is.na(x)]", "equals_na"));
        assert!(no_lint("# x == NA", "equals_na"));
        assert!(no_lint("'x == NA'", "equals_na"));
        assert!(no_lint("x == f(NA)", "equals_na"));
    }
}
