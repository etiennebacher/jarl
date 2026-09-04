pub(crate) mod deparse1;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "deparse1", Some("4.1.0"))
    }

    #[test]
    fn test_no_lint_deparse1() {
        expect_no_lint("deparse1(x)", "deparse1", Some("4.1.0"));
        expect_no_lint("deparse(x)", "deparse1", Some("4.1.0"));
        // no collapse=
        expect_no_lint("paste(deparse(x))", "deparse1", Some("4.1.0"));
        // multiple positional args to paste
        expect_no_lint(
            "paste('Error: ', deparse(x), collapse = ' ')",
            "deparse1",
            Some("4.1.0"),
        );
        // 'deparse' as a symbol, not a call
        expect_no_lint("paste(deparse, collapse = ' ')", "deparse1", Some("4.1.0"));
        // deparse() supplies the separator, i.e. it is not the collapsed vector
        expect_no_lint(
            "paste(x, collapse = deparse(sep))",
            "deparse1",
            Some("4.1.0"),
        );
        // collapse = NULL does not collapse, so it keeps deparse()'s multi-element output
        expect_no_lint(
            "paste(deparse(x), collapse = NULL)",
            "deparse1",
            Some("4.1.0"),
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
          | --------------------------------- `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
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
          | --------------------------------------------- `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
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
          | ------------------------------------------------------ `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
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
          | --------------------------------- `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
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
          | --------------------------------------------- `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
          |
          = help: Use `deparse1(x)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "paste(deparse(x), collapse = ' ')",
                    "paste(deparse(substitute(x)), collapse = ' ')",
                    "paste(deparse(x, width.cutoff = 500L), collapse = ' ')",
                    "paste(collapse = ' ', deparse(x))",
                    "base::paste(base::deparse(x), collapse = ' ')",
                ],
                "deparse1",
                Some("4.1.0")
            )
        );
    }

    #[test]
    fn test_lint_deparse1_vectorizes() {
        // the recommendation applies independently to every matching call, so it
        // vectorizes over multiple statements
        assert_snapshot!(
            snapshot_lint(
                "{\n  paste(deparse(x), collapse = ' ')\n  paste(deparse(y), collapse = ' ')\n}"
            ),
            @r#"
        warning: deparse1
         --> <test>:2:3
          |
        2 |   paste(deparse(x), collapse = ' ')
          |   --------------------------------- `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
          |
          = help: Use `deparse1(x)` instead.
        warning: deparse1
         --> <test>:3:3
          |
        3 |   paste(deparse(y), collapse = ' ')
          |   --------------------------------- `paste(deparse(x), collapse = " ")` is inefficient and can be hard to read.
          |
          = help: Use `deparse1(x)` instead.
        Found 2 errors.
        "#
        );
    }
}
