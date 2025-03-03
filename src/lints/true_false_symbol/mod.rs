pub(crate) mod true_false_symbol;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_true_false_symbol() {
        use insta::assert_snapshot;
        let (lint_output, fix_output) = get_lint_and_fix_text(
            vec![
                "T",
                "F",
                "T = 42",
                "F = 42",
                "for (i in 1:10) {x <- c(T, TRUE, F, FALSE)}",
                "DF$bool <- T",
                "S4@bool <- T",
                "sum(x, na.rm = T)",
            ],
            "true_false_symbol",
        );
        assert_snapshot!("lint_output", lint_output);
        assert_snapshot!("fix_output", fix_output);
    }

    #[test]
    fn test_no_lint_true_false_symbol() {
        assert!(no_lint("TRUE", "true_false_symbol",));
        assert!(no_lint("FALSE", "true_false_symbol",));
        assert!(no_lint("T()", "true_false_symbol",));
        assert!(no_lint("F()", "true_false_symbol",));
        assert!(no_lint("x <- \"T\"", "true_false_symbol",));
    }
    #[test]
    fn test_true_false_symbol_in_formulas() {
        use insta::assert_snapshot;
        let (lint_output, fix_output) = get_lint_and_fix_text(
            vec!["lm(weight ~ var + foo(x, arg = T), data)"],
            "true_false_symbol",
        );
        assert_snapshot!("lint_output", lint_output);
        assert_snapshot!("fix_output", fix_output);

        assert!(no_lint("lm(weight ~ T, data)", "true_false_symbol"));
        assert!(no_lint("lm(weight ~ F, data)", "true_false_symbol"));
        assert!(no_lint("lm(weight ~ T + var", "true_false_symbol"));
        assert!(no_lint("lm(weight ~ A + T | var", "true_false_symbol"));
        assert!(no_lint("lm(weight ~ var | A + T", "true_false_symbol"));
        assert!(no_lint(
            "lm(weight ~ var + var2 + T, data)",
            "true_false_symbol"
        ));
        assert!(no_lint("lm(T ~ weight, data)", "true_false_symbol"));
    }
}
