pub(crate) mod sample_int;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "sample_int", None)
    }

    #[test]
    fn test_no_lint_sample_int() {
        expect_no_lint("sample('a', m)", "sample_int", None);
        expect_no_lint("sample(1, m)", "sample_int", None);
        expect_no_lint("sample(n, m)", "sample_int", None);
        expect_no_lint("sample(n, m, TRUE)", "sample_int", None);
        expect_no_lint("sample(n, m, prob = 1:n/n)", "sample_int", None);
        expect_no_lint("sample(foo(x), m, TRUE)", "sample_int", None);
        expect_no_lint("sample(n, replace = TRUE)", "sample_int", None);
        expect_no_lint("sample(10:1, m)", "sample_int", None);
        expect_no_lint("sample(replace = TRUE, letters)", "sample_int", None);
        expect_no_lint("x$sample(1:2, 1)", "sample_int", None);
    }

    #[test]
    fn test_lint_sample_int() {
        assert_snapshot!(
            snapshot_lint("sample(1:10, 2)"),
            @r"
        warning: sample_int
         --> <test>:1:1
          |
        1 | sample(1:10, 2)
          | --------------- `sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.
          |
          = help: Use `sample.int(n, m, ...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sample(1L:10L, 2)"),
            @r"
        warning: sample_int
         --> <test>:1:1
          |
        1 | sample(1L:10L, 2)
          | ----------------- `sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.
          |
          = help: Use `sample.int(n, m, ...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sample(1:n, 2)"),
            @r"
        warning: sample_int
         --> <test>:1:1
          |
        1 | sample(1:n, 2)
          | -------------- `sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.
          |
          = help: Use `sample.int(n, m, ...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sample(1:k, replace = TRUE)"),
            @r"
        warning: sample_int
         --> <test>:1:1
          |
        1 | sample(1:k, replace = TRUE)
          | --------------------------- `sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.
          |
          = help: Use `sample.int(n, m, ...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sample(1:foo(x), prob = bar(x))"),
            @r"
        warning: sample_int
         --> <test>:1:1
          |
        1 | sample(1:foo(x), prob = bar(x))
          | ------------------------------- `sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.
          |
          = help: Use `sample.int(n, m, ...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "sample(1:10, 2)",
                    "sample(1L:10L, 2)",
                    "sample(n = 1:10, 2)",
                    "sample(2, n = 1:10)",
                    "sample(size = 2, n = 1:10)",
                    "sample(replace = TRUE, letters)",
                ],
                "sample_int",
                None
            )
        );
    }

    #[test]
    fn test_sample_int_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsample(1:10, 2)",
                    "sample(\n  # comment\n  1:10, 2\n)",
                    "sample(1:n,\n    # comment\n    2)",
                    "sample(1:10, 2) # trailing comment",
                ],
                "sample_int",
                None
            )
        );
    }
}
