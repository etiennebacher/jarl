pub(crate) mod deparse1;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "deparse1", Some("4.0.0"))
    }

    #[test]
    fn test_no_lint_deparse1() {
        expect_no_lint("deparse1(x)", "deparse1", Some("4.0.0"));
        expect_no_lint("deparse(x)", "deparse1", Some("4.0.0"));
        // no collapse=
        expect_no_lint("paste(deparse(x))", "deparse1", Some("4.0.0"));
        // multiple positional args to paste
        expect_no_lint(
            "paste('Error: ', deparse(x), collapse = ' ')",
            "deparse1",
            Some("4.0.0"),
        );
        // 'deparse' as a symbol, not a call
        expect_no_lint("paste(deparse, collapse = ' ')", "deparse1", Some("4.0.0"));
        // deparse() supplies the separator, i.e. it is not the collapsed vector
        expect_no_lint(
            "paste(x, collapse = deparse(sep))",
            "deparse1",
            Some("4.0.0"),
        );
        // collapse = NULL does not collapse, so it keeps deparse()'s multi-element output
        expect_no_lint(
            "paste(deparse(x), collapse = NULL)",
            "deparse1",
            Some("4.0.0"),
        );
    }

    #[test]
    fn test_lint_deparse1() {
        assert_snapshot!(
            snapshot_lint("paste(deparse(x), collapse = ' ')"),
            @r#"
        warning: deparse1
         --> <test>:1:1
          |
        1 | paste(deparse(x), collapse = ' ')
          | --------------------------------- `paste(deparse(x), collapse = " ")` can be simplified.
          |
          = help: Use `deparse1(x)` instead.
        Found 1 error.
        "#
        );

        // nested inner call
        assert_snapshot!(
            snapshot_lint("paste(deparse(substitute(x)), collapse = ' ')"),
            @r#"
        warning: deparse1
         --> <test>:1:1
          |
        1 | paste(deparse(substitute(x)), collapse = ' ')
          | --------------------------------------------- `paste(deparse(x), collapse = " ")` can be simplified.
          |
          = help: Use `deparse1(x)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("paste(deparse(x, width.cutoff = 500L), collapse = ' ')"),
            @r#"
        warning: deparse1
         --> <test>:1:1
          |
        1 | paste(deparse(x, width.cutoff = 500L), collapse = ' ')
          | ------------------------------------------------------ `paste(deparse(x), collapse = " ")` can be simplified.
          |
          = help: Use `deparse1(x)` instead.
        Found 1 error.
        "#
        );

        // arg order doesn't matter
        assert_snapshot!(
            snapshot_lint("paste(collapse = ' ', deparse(x))"),
            @r#"
        warning: deparse1
         --> <test>:1:1
          |
        1 | paste(collapse = ' ', deparse(x))
          | --------------------------------- `paste(deparse(x), collapse = " ")` can be simplified.
          |
          = help: Use `deparse1(x)` instead.
        Found 1 error.
        "#
        );

        // namespace-qualified
        assert_snapshot!(
            snapshot_lint("base::paste(base::deparse(x), collapse = ' ')"),
            @r#"
        warning: deparse1
         --> <test>:1:1
          |
        1 | base::paste(base::deparse(x), collapse = ' ')
          | --------------------------------------------- `paste(deparse(x), collapse = " ")` can be simplified.
          |
          = help: Use `deparse1(x)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "paste(deparse(x), collapse = ' ')",
                    "paste(deparse(substitute(x)), collapse = ' ')",
                    "paste(deparse(x, width.cutoff = 500L), collapse = ' ')",
                    "paste(collapse = ' ', deparse(x))",
                    "base::paste(base::deparse(x), collapse = ' ')",
                ],
                "deparse1",
                Some("4.0.0"),
                None
            )
        );
    }

    #[test]
    fn test_deparse1_with_comments_no_fix() {
        // Leading and trailing comments are outside the replaced range, so the
        // fix is applied. Comments inside the expression would be lost, so the
        // lint is reported but no fix is applied.
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "# leading comment\npaste(deparse(x), collapse = ' ')",
                    "paste(deparse(x), collapse = ' ') # trailing comment",
                    "paste(\n  # comment\n  deparse(x),\n  collapse = ' '\n)",
                    "paste(deparse(\n  # comment\n  x\n), collapse = ' ')",
                ],
                "deparse1",
                Some("4.0.0"),
                None
            )
        );
    }
}
