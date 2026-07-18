pub(crate) mod rep_len;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "rep_len", None)
    }

    #[test]
    fn test_no_lint_rep_len() {
        expect_no_lint("rep(x, y)", "rep_len", None);
        expect_no_lint("rep(1:10, 2)", "rep_len", None);
        expect_no_lint("rep(1:10, 10:1)", "rep_len", None);
        expect_no_lint("rep(x = 1:10, 50)", "rep_len", None);
        expect_no_lint("rep(times = 2, 50)", "rep_len", None);
        expect_no_lint("rep(x, each = 4, length.out = 50)", "rep_len", None);
        expect_no_lint("rep(x, eac = 4, length.out = 50)", "rep_len", None);
        expect_no_lint("rep(x, other = 4, length.out = 50)", "rep_len", None);
        expect_no_lint("rep(x = x, x = y, length.out = 50)", "rep_len", None);
        expect_no_lint("rep(a, b, length.out = c, d)", "rep_len", None);
        expect_no_lint("rep(a, b, c, d)", "rep_len", None);
        expect_no_lint("rep(x, length.out =)", "rep_len", None);
        expect_no_lint("rep_len(x, 10)", "rep_len", None);
    }

    #[test]
    fn test_lint_rep_len() {
        assert_snapshot!(
            snapshot_lint("rep(x, length.out = 4L)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(x, length.out = 4L)
          | ----------------------- `rep_len(x, 4L)` is more explicit than `rep(x, length.out = 4L)`.
          |
          = help: Use `rep_len(x, 4L)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::rep(length.out = 50, x = 1:10)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | base::rep(length.out = 50, x = 1:10)
          | ------------------------------------ `base::rep_len(1:10, 50)` is more explicit than `base::rep(length.out = 50, x = 1:10)`.
          |
          = help: Use `base::rep_len(1:10, 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(length.out = 50, 1:10)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(length.out = 50, 1:10)
          | -------------------------- `rep_len(1:10, 50)` is more explicit than `rep(length.out = 50, 1:10)`.
          |
          = help: Use `rep_len(1:10, 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:10, 10:1, length.out = 50)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(1:10, 10:1, length.out = 50)
          | -------------------------------- `rep_len(1:10, 50)` is more explicit than `rep(1:10, 10:1, length.out = 50)`.
          |
          = help: Use `rep_len(1:10, 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:10, times = 10:1, length.out = 50)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(1:10, times = 10:1, length.out = 50)
          | ---------------------------------------- `rep_len(1:10, 50)` is more explicit than `rep(1:10, times = 10:1, length.out = 50)`.
          |
          = help: Use `rep_len(1:10, 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:10, 10:1, 50)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(1:10, 10:1, 50)
          | ------------------- `rep_len(1:10, 50)` is more explicit than `rep(1:10, 10:1, 50)`.
          |
          = help: Use `rep_len(1:10, 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(x = 1:2, 2, 50)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(x = 1:2, 2, 50)
          | ------------------- `rep_len(1:2, 50)` is more explicit than `rep(x = 1:2, 2, 50)`.
          |
          = help: Use `rep_len(1:2, 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:2, times = 2, 50)"),
            @"
        warning: rep_len
         --> <test>:1:1
          |
        1 | rep(1:2, times = 2, 50)
          | ----------------------- `rep_len(1:2, 50)` is more explicit than `rep(1:2, times = 2, 50)`.
          |
          = help: Use `rep_len(1:2, 50)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_rep_len() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "rep(x, length.out = 4L)",
                    "base::rep(length.out = 50, x = 1:10)",
                    "rep(length.out = 50, 1:10)",
                    "rep(1:10, 10:1, length.out = 50)",
                    "rep(1:10, times = 10:1, length.out = 50)",
                    "rep(1:10, 10:1, 50)",
                    "rep(x = 1:2, 2, 50)",
                    "rep(1:2, times = 2, 50)",
                ],
                "rep_len",
            )
        );
    }

    #[test]
    fn test_rep_len_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nrep(x, length.out = 4L)",
                    "rep(x,\n  # comment\n  length.out = 4L)",
                    "rep(x, length.out = 4L) # trailing comment",
                ],
                "rep_len",
            )
        );
    }
}
