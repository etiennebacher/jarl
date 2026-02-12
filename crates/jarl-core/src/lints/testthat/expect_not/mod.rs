pub(crate) mod expect_not;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_not", None)
    }

    #[test]
    fn test_no_lint_expect_not() {
        expect_no_lint("expect_true()", "expect_not", None);
        expect_no_lint("expect_false()", "expect_not", None);

        // Allowed usages without negation
        expect_no_lint("expect_true(x)", "expect_not", None);
        expect_no_lint("expect_false(x)", "expect_not", None);

        // Not a strict ban on ! - complex boolean expressions are allowed
        expect_no_lint("expect_true(!x || !y)", "expect_not", None);

        // Not the functions we're looking for
        expect_no_lint("some_other_function(!x)", "expect_not", None);
        expect_no_lint("expect_true(~x)", "expect_not", None);

        // rlang "!!" and "!!!" operators should not be flagged
        expect_no_lint("expect_true(!!x)", "expect_not", None);
        expect_no_lint("expect_true(!!!x)", "expect_not", None);
        expect_no_lint("expect_false(!!x)", "expect_not", None);
        expect_no_lint("expect_false(!!!x)", "expect_not", None);

        // Named arguments in different positions
        expect_no_lint(
            "expect_true(label = 'test', object = !x)",
            "expect_not",
            None,
        );

        // Wrong syntax but no panic
        expect_no_lint("expect_true(!)", "expect_not", None);
        expect_no_lint("expect_true(!!)", "expect_not", None);
        expect_no_lint("expect_true(!!!)", "expect_not", None);
        expect_no_lint("expect_true(object =)", "expect_not", None);
        expect_no_lint("expect_true(~)", "expect_not", None);
    }

    #[test]
    fn test_lint_expect_not() {
        assert_snapshot!(
            snapshot_lint("expect_true(!x)"),
            @r"
        warning: expect_not
         --> <test>:1:1
          |
        1 | expect_true(!x)
          | --------------- `expect_true(!x)` is not as clear as `expect_false(x)`.
          |
          = help: Use `expect_false(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_true(!x)"),
            @r"
        warning: expect_not
         --> <test>:1:1
          |
        1 | testthat::expect_true(!x)
          | ------------------------- `expect_true(!x)` is not as clear as `expect_false(x)`.
          |
          = help: Use `expect_false(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_false(!foo(x))"),
            @r"
        warning: expect_not
         --> <test>:1:1
          |
        1 | expect_false(!foo(x))
          | --------------------- `expect_false(!x)` is not as clear as `expect_true(x)`.
          |
          = help: Use `expect_true(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(!(x && y))"),
            @r"
        warning: expect_not
         --> <test>:1:1
          |
        1 | expect_true(!(x && y))
          | ---------------------- `expect_true(!x)` is not as clear as `expect_false(x)`.
          |
          = help: Use `expect_false(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_false(!is.na(x))"),
            @r"
        warning: expect_not
         --> <test>:1:1
          |
        1 | expect_false(!is.na(x))
          | ----------------------- `expect_false(!x)` is not as clear as `expect_true(x)`.
          |
          = help: Use `expect_true(x)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_true(!x)",
                    "testthat::expect_true(!x)",
                    "expect_false(!foo(x))",
                    "expect_true(!(x && y))",
                    "expect_true(!(x && (y || foo(x))))",
                    "expect_false(!is.na(x))",
                ],
                "expect_not",
                None,
            )
        );
    }

    #[test]
    fn test_expect_not_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_true(!x)",
                    "expect_false(# comment\n!x)",
                    "expect_true(!x) # trailing comment",
                ],
                "expect_not",
                None
            )
        );
    }
}
