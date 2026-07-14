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
          |           ---------- `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("stopifnot(check = all(x))"),
            @r"
        warning: stopifnot_all
         --> <test>:1:19
          |
        1 | stopifnot(check = all(x))
          |                   ------ `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::stopifnot(base::all(x))"),
            @r"
        warning: stopifnot_all
         --> <test>:1:17
          |
        1 | base::stopifnot(base::all(x))
          |                 ------------ `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stopifnot(all(x, na.rm = TRUE))"),
            @r"
        warning: stopifnot_all
         --> <test>:1:11
          |
        1 | stopifnot(all(x, na.rm = TRUE))
          |           -------------------- `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stopifnot(all(x, y))"),
            @r"
        warning: stopifnot_all
         --> <test>:1:11
          |
        1 | stopifnot(all(x, y))
          |           --------- `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stopifnot(x, all(y), all(z))"),
            @r"
        warning: stopifnot_all
         --> <test>:1:14
          |
        1 | stopifnot(x, all(y), all(z))
          |              ------ `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        warning: stopifnot_all
         --> <test>:1:22
          |
        1 | stopifnot(x, all(y), all(z))
          |                      ------ `stopifnot(all(...))` contains an unnecessary call to `all()`.
          |
          = help: Use `stopifnot(...)` instead.
        Found 2 errors.
        "
        );
    }
}
