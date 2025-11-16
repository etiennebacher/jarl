pub(crate) mod outer_negation;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_outer_negation() {
        expect_no_lint("x <- any(y)", "outer_negation", None);
        expect_no_lint("y <- all(z)", "outer_negation", None);

        // extended usage of any is not covered
        expect_no_lint("any(!a & b)", "outer_negation", None);
        expect_no_lint("all(a | !b)", "outer_negation", None);

        expect_no_lint("any(a, b)", "outer_negation", None);
        expect_no_lint("all(b, c)", "outer_negation", None);
        expect_no_lint("any(!a, b)", "outer_negation", None);
        expect_no_lint("any(!!a)", "outer_negation", None);
        expect_no_lint("any(!!!a)", "outer_negation", None);
        expect_no_lint("all(a, !b)", "outer_negation", None);
        expect_no_lint("any(a, !b, na.rm = TRUE)", "outer_negation", None);
        // ditto when na.rm is passed quoted
        expect_no_lint("any(a, !b, 'na.rm' = TRUE)", "outer_negation", None);
    }

    #[test]
    fn test_lint_outer_negation() {
        use insta::assert_snapshot;

        expect_lint("any(!x)", "Use `!all(x)` instead", "outer_negation", None);
        expect_lint(
            "any(!(x + y))",
            "Use `!all(x)` instead",
            "outer_negation",
            None,
        );
        expect_lint(
            "all(!foo(x))",
            "Use `!any(x)` instead",
            "outer_negation",
            None,
        );
        expect_lint(
            "all(!(x + y))",
            "Use `!any(x)` instead",
            "outer_negation",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(!x)",
                    "any(!f(x, y))",
                    "any(!f(all(!x)))",
                    "all(!x)",
                    "all(!f(x, y))",
                    "all(!f(any(!x)))"
                ],
                "outer_negation",
                None
            )
        );
    }

    #[test]
    fn test_outer_negation_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nany(!x)",
                    "any(\n # comment\n !x\n)",
                    "all(\n # comment\n !x\n)",
                    "any(!x) # trailing comment",
                ],
                "outer_negation",
                None
            )
        );
    }
}
