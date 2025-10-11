pub(crate) mod length_levels;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_length_levels() {
        use insta::assert_snapshot;
        let expected_message = "Use `nlevels(...)` instead";

        expect_lint(
            "2:length(levels(x))",
            expected_message,
            "length_levels",
            None,
        );
        expect_lint(
            "2:length(levels(foo(a)))",
            expected_message,
            "length_levels",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec!["2:length(levels(x))", "2:length(levels(foo(a)))",],
                "length_levels",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_length_levels() {
        expect_no_lint("length(c(levels(x), 'a'))", "length_levels", None);
    }

    #[test]
    fn test_length_levels_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nlength(levels(x))",
                    "length(\n  # comment\n  levels(x)\n)",
                    "length(levels(\n    # comment\n    x\n  ))",
                    "length(levels(x)) # trailing comment",
                ],
                "length_levels",
                None
            )
        );
    }
}
