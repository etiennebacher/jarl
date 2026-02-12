pub(crate) mod expect_length;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_length", None)
    }

    #[test]
    fn test_no_lint_expect_length() {
        expect_no_lint("expect_equal(nrow(x), 4L)", "expect_length", None);
        expect_no_lint("expect_identical(nrow(x), 4L)", "expect_length", None);

        // expect_length() doesn't have info= or label= arguments
        expect_no_lint(
            "expect_equal(length(x), n, info = 'x should have size n')",
            "expect_length",
            None,
        );
        expect_no_lint(
            "expect_equal(length(x), n, label = 'x size')",
            "expect_length",
            None,
        );
        expect_no_lint(
            "expect_equal(length(x), n, expected.label = 'target size')",
            "expect_length",
            None,
        );
        expect_no_lint("expect_equal(length(x), length(y))", "expect_length", None);
        expect_no_lint("expect_equal(foo(x), bar(y))", "expect_length", None);

        // Not the functions we're looking for
        expect_no_lint("expect_equal(x, 1)", "expect_length", None);
        expect_no_lint("some_other_function(length(x), n)", "expect_length", None);

        // Wrong code but no panic
        expect_no_lint("expect_equal(length(x))", "expect_length", None);
        expect_no_lint("expect_equal(length())", "expect_length", None);
        expect_no_lint("expect_equal(object =, expected =)", "expect_length", None);
    }

    #[test]
    fn test_lint_expect_length() {
        assert_snapshot!(
            snapshot_lint("expect_equal(length(x), 2)"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | expect_equal(length(x), 2)
          | -------------------------- `expect_length(x, n)` is better than `expect_equal(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_equal(length(x), 2)"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | testthat::expect_equal(length(x), 2)
          | ------------------------------------ `expect_length(x, n)` is better than `expect_equal(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_identical(length(x), 2)"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | expect_identical(length(x), 2)
          | ------------------------------ `expect_length(x, n)` is better than `expect_identical(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(2, length(x))"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | expect_equal(2, length(x))
          | -------------------------- `expect_length(x, n)` is better than `expect_equal(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(2L, length(x))"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | expect_equal(2L, length(x))
          | --------------------------- `expect_length(x, n)` is better than `expect_equal(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(foo(y), length(x))"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | expect_equal(foo(y), length(x))
          | ------------------------------- `expect_length(x, n)` is better than `expect_equal(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(length(x), 2L)",
                    "expect_equal(2, length(x))",
                    "expect_equal(length(x), foo(y))",
                    "expect_equal(foo(y), length(x))",
                    "testthat::expect_equal(base::length(x), 2)",
                ],
                "expect_length",
                None,
            )
        );
    }

    #[test]
    fn test_expect_length_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present
        assert_snapshot!(
            snapshot_lint("expect_equal(# comment\nlength(x), 2L)"),
            @r"
        warning: expect_length
         --> <test>:1:1
          |
        1 | / expect_equal(# comment
        2 | | length(x), 2L)
          | |______________- `expect_length(x, n)` is better than `expect_equal(length(x), n)`.
          |
          = help: Use `expect_length(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(length(x), 2L)",
                    "expect_equal(# comment\nlength(x), 2L)",
                    "expect_equal(length(x), 2L) # trailing comment",
                ],
                "expect_length",
                None
            )
        );
    }
}
