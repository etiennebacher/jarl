pub(crate) mod length_levels;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "length_levels", None)
    }

    #[test]
    fn test_lint_length_levels() {
        assert_snapshot!(
            snapshot_lint("2:length(levels(x))"),
            @r"
        warning: length_levels
         --> <test>:1:3
          |
        1 | 2:length(levels(x))
          |   ----------------- `length(levels(...))` is less readable than `nlevels(...)`.
          |
          = help: Use `nlevels(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("2:length(levels(foo(a)))"),
            @r"
        warning: length_levels
         --> <test>:1:3
          |
        1 | 2:length(levels(foo(a)))
          |   ---------------------- `length(levels(...))` is less readable than `nlevels(...)`.
          |
          = help: Use `nlevels(...)` instead.
        Found 1 error.
        "
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
