pub(crate) mod stopifnot_all;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "stopifnot_all", None)
    }

    #[test]
    fn test_no_lint_stopifnot_all() {
        expect_no_lint("all(x)", "stopifnot_all", None);
        expect_no_lint("stopifnot(x)", "stopifnot_all", None);
        expect_no_lint("assert_that(all(x))", "stopifnot_all", None);
        expect_no_lint("stopifnot(all(x) || any(y))", "stopifnot_all", None);
        expect_no_lint("stopifnot(foo(all(x)))", "stopifnot_all", None);
        expect_no_lint("stopifnot((all(x)))", "stopifnot_all", None);
        expect_no_lint("stopifnot(all(x)[1])", "stopifnot_all", None);
        expect_no_lint("stopifnot(all = x)", "stopifnot_all", None);
    }

    #[test]
    fn test_lint_stopifnot_all() {
        assert_snapshot!(
            snapshot_lint("stopifnot(all(x > 0))"),
            @r"
        warning: stopifnot_all
         --> <test>:1:11
          |
        1 | stopifnot(all(x > 0))
          |           ---------- `stopifnot(all(x))` produces a less informative error message.
          |
          = help: Use `stopifnot(x)` instead.
        Found 1 error.
        "
        );

        for code in [
            "stopifnot(check = all(x))",
            "base::stopifnot(base::all(x))",
            "stopifnot(all(x, na.rm = TRUE))",
            "stopifnot(all(x, y))",
        ] {
            assert_eq!(
                check_code(code, "stopifnot_all", None).len(),
                1,
                "expected a lint for `{code}`"
            );
        }

        assert_eq!(
            check_code("stopifnot(x, all(y), all(z))", "stopifnot_all", None).len(),
            2
        );
    }
}
