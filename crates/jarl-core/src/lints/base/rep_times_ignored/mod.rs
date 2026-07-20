pub(crate) mod rep_times_ignored;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "rep_times_ignored", None)
    }

    #[test]
    fn test_no_lint_rep_times_ignored() {
        expect_no_lint("rep(x, y)", "rep_times_ignored", None);
        expect_no_lint("rep(1:10, 2)", "rep_times_ignored", None);
        expect_no_lint("rep(1:10, 10:1)", "rep_times_ignored", None);
        expect_no_lint("rep(x, length.out = 4L)", "rep_times_ignored", None);
        expect_no_lint(
            "base::rep(length.out = 50, x = 1:10)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint("rep(length.out = 50, 1:10)", "rep_times_ignored", None);
        expect_no_lint("rep(times = 2, 50)", "rep_times_ignored", None);
        expect_no_lint(
            "rep(x, each = 4, length.out = 50)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint(
            "rep(x, eac = 4, length.out = 50)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint(
            "rep(x, other = 4, length.out = 50)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint(
            "rep(x = x, x = y, length.out = 50)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint("rep(a, b, c, d, e)", "rep_times_ignored", None);
        expect_no_lint(
            "rep(x, times =, length.out = 50)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint("rep(x, length.out =)", "rep_times_ignored", None);
        expect_no_lint(
            "rep(x, times = 2, length.out = NA)",
            "rep_times_ignored",
            None,
        );
        expect_no_lint("rep_len(x, 10)", "rep_times_ignored", None);
    }

    #[test]
    fn test_lint_rep_times_ignored() {
        assert_snapshot!(
            snapshot_lint("rep(1:10, 10:1, length.out = 50)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(1:10, 10:1, length.out = 50)
          | -------------------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(1:10, length.out = 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:10, times = 10:1, length.out = 50)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(1:10, times = 10:1, length.out = 50)
          | ---------------------------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(1:10, length.out = 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:10, 10:1, 50)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(1:10, 10:1, 50)
          | ------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(1:10, length.out = 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(x = 1:2, 2, 50)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(x = 1:2, 2, 50)
          | ------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(1:2, length.out = 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(1:2, times = 2, 50)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(1:2, times = 2, 50)
          | ----------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(1:2, length.out = 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::rep(length.out = 50, times = 2, x = 1:2)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | base::rep(length.out = 50, times = 2, x = 1:2)
          | ---------------------------------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `base::rep(1:2, length.out = 50)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(x, times = 2, length.out = 5, each = 3)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(x, times = 2, length.out = 5, each = 3)
          | ------------------------------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(x, length.out = 5, each = 3)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(x, 2, 5, 3)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(x, 2, 5, 3)
          | --------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(x, length.out = 5, each = 3)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(each = 3, x, length.out = 5, times = 2)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(each = 3, x, length.out = 5, times = 2)
          | ------------------------------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(x, length.out = 5, each = 3)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rep(x, times = 2, length.out = 5, each =)"),
            @"
        warning: rep_times_ignored
         --> <test>:1:1
          |
        1 | rep(x, times = 2, length.out = 5, each =)
          | ----------------------------------------- `times` is ignored when `length.out` is supplied.
          |
          = help: Use `rep(x, length.out = 5)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_rep_times_ignored() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "rep(1:10, 10:1, length.out = 50)",
                    "rep(1:10, times = 10:1, length.out = 50)",
                    "rep(1:10, 10:1, 50)",
                    "rep(x = 1:2, 2, 50)",
                    "rep(1:2, times = 2, 50)",
                    "base::rep(length.out = 50, times = 2, x = 1:2)",
                    "rep(x, times = 2, length.out = 5, each = 3)",
                    "rep(x, 2, 5, 3)",
                    "rep(each = 3, x, length.out = 5, times = 2)",
                    "rep(x, times = 2, length.out = 5, each =)",
                ],
                "rep_times_ignored",
            )
        );
    }

    #[test]
    fn test_rep_times_ignored_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nrep(x, times = 2, length.out = 4L)",
                    "rep(x,\n  times = 2,\n  # comment\n  length.out = 4L)",
                    "rep(x, times = 2, length.out = 4L) # trailing comment",
                ],
                "rep_times_ignored",
            )
        );
    }
}
