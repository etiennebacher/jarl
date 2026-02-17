pub(crate) mod all_equal;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "all_equal", None)
    }

    #[test]
    fn test_no_lint_all_equal() {
        expect_no_lint("all.equal(a, b)", "all_equal", None);
        expect_no_lint("all.equal(a, b, tolerance = 1e-3)", "all_equal", None);
        expect_no_lint("if (isFALSE(x)) 1", "all_equal", None);
        expect_no_lint(
            "if (isTRUE(all.equal(a, b))) message('equal')",
            "all_equal",
            None,
        );
        expect_no_lint(
            "if (!isTRUE(all.equal(a, b))) message('different')",
            "all_equal",
            None,
        );
        expect_no_lint("if (A) all.equal(x, y)", "all_equal", None);
        // Incomplete pipe chains should not trigger
        expect_no_lint("all.equal(a, b) |> isTRUE()", "all_equal", None);
        expect_no_lint("all.equal(a, b) |> mean() |> isTRUE()", "all_equal", None);
        expect_no_lint("x |> isFALSE()", "all_equal", None);
    }

    #[test]
    fn test_lint_all_equal() {
        assert_snapshot!(
            snapshot_lint("if (all.equal(a, b, tolerance = 1e-3)) message('equal')"),
            @r"
        warning: all_equal
         --> <test>:1:5
          |
        1 | if (all.equal(a, b, tolerance = 1e-3)) message('equal')
          |     --------------------------------- If `all.equal()` is false, it will return a string and not `FALSE`.
          |
          = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (all.equal(a, b)) message('equal')"),
            @r"
        warning: all_equal
         --> <test>:1:5
          |
        1 | if (all.equal(a, b)) message('equal')
          |     --------------- If `all.equal()` is false, it will return a string and not `FALSE`.
          |
          = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!all.equal(a, b)"),
            @r"
        warning: all_equal
         --> <test>:1:1
          |
        1 | !all.equal(a, b)
          | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
          |
          = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (all.equal(a, b)) message('equal')"),
            @r"
        warning: all_equal
         --> <test>:1:8
          |
        1 | while (all.equal(a, b)) message('equal')
          |        --------------- If `all.equal()` is false, it will return a string and not `FALSE`.
          |
          = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("isFALSE(all.equal(a, b))"),
            @r"
        warning: all_equal
         --> <test>:1:1
          |
        1 | isFALSE(all.equal(a, b))
          | ------------------------ `isFALSE(all.equal())` always returns `FALSE`
          |
          = help: Use `!isTRUE()` to check for differences instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "if (all.equal(a, b, tolerance = 1e-3)) message('equal')",
                    "if (all.equal(a, b)) message('equal')",
                    "!all.equal(a, b)",
                    "while (all.equal(a, b)) message('equal')",
                    "isFALSE(all.equal(a, b))",
                    "if (
  # A comment
  all.equal(a, b)
) message('equal')",
                ],
                "all_equal",
            )
        );
    }

    #[test]
    fn test_lint_all_equal_piped() {
        assert_snapshot!(
            snapshot_lint("all.equal(a, b) |> \n isFALSE()"),
            @r"
        warning: all_equal
         --> <test>:1:1
          |
        1 | / all.equal(a, b) |> 
        2 | |  isFALSE()
          | |__________- `isFALSE(all.equal())` always returns `FALSE`
          |
          = help: Use `!isTRUE()` to check for differences instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "multiline_pipe",
            get_unsafe_fixed_text(vec!["all.equal(a, b) |>\n  isFALSE()"], "all_equal",)
        );
    }

    #[test]
    fn test_all_equal_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nif (all.equal(a, b)) message('equal')",
                    "if (all.equal(a,\n# a comment\n b)) message('equal')",
                    "if (all.equal(a, b)) message('equal') # trailing comment",
                ],
                "all_equal",
            )
        );
    }
}
