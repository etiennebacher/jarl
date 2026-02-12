pub(crate) mod expect_type;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_type", None)
    }

    #[test]
    fn test_no_lint_expect_type() {
        // expect_type doesn't have an inverted version
        expect_no_lint("expect_true(!is.numeric(x))", "expect_type", None);

        // other is.<x> calls are not suitable for expect_type in particular
        expect_no_lint("expect_true(is.data.frame(x))", "expect_type", None);

        // expect_type(x, ...) cannot be cleanly used here:
        expect_no_lint(
            "expect_true(typeof(x) %in% c('builtin', 'closure'))",
            "expect_type",
            None,
        );

        // expect_type() doesn't have info= or label= arguments
        expect_no_lint(
            "expect_equal(typeof(x), t, info = 'x should have type t')",
            "expect_type",
            None,
        );
        expect_no_lint(
            "expect_equal(typeof(x), t, label = 'x type')",
            "expect_type",
            None,
        );
        expect_no_lint(
            "expect_equal(typeof(x), t, expected.label = 'type')",
            "expect_type",
            None,
        );
        expect_no_lint(
            "expect_true(is.double(x), info = 'x should be double')",
            "expect_type",
            None,
        );

        // Not the functions we're looking for
        expect_no_lint("expect_false(is.integer(x))", "expect_type", None);
        expect_no_lint("expect_equal(class(x), 'foo')", "expect_type", None);
        expect_no_lint("expect_true(is.numeric(x))", "expect_type", None);
        expect_no_lint("expect_true(foo(x))", "expect_type", None);

        // Wrong code but no panic
        expect_no_lint("expect_equal(typeof())", "expect_type", None);
        expect_no_lint("expect_equal(typeof(x))", "expect_type", None);
        expect_no_lint("expect_true(is.integer())", "expect_type", None);
        expect_no_lint("expect_true(is.integer(x =))", "expect_type", None);
    }

    #[test]
    fn test_lint_expect_type() {
        assert_snapshot!(
            snapshot_lint("expect_equal(typeof(x), 'double')"),
            @r"
        warning: expect_type
         --> <test>:1:1
          |
        1 | expect_equal(typeof(x), 'double')
          | --------------------------------- `expect_equal(typeof(x), t)` can be hard to read.
          |
          = help: Use `expect_type(x, t)` instead.
        Found 1 error.
        "
        );
        // expect_identical is treated the same as expect_equal
        assert_snapshot!(
            snapshot_lint("testthat::expect_identical(typeof(x), 'language')"),
            @r"
        warning: expect_type
         --> <test>:1:1
          |
        1 | testthat::expect_identical(typeof(x), 'language')
          | ------------------------------------------------- `expect_identical(typeof(x), t)` can be hard to read.
          |
          = help: Use `expect_type(x, t)` instead.
        Found 1 error.
        "
        );
        // different equivalent usage
        assert_snapshot!(
            snapshot_lint("expect_true(is.complex(foo(x)))"),
            @r"
        warning: expect_type
         --> <test>:1:1
          |
        1 | expect_true(is.complex(foo(x)))
          | ------------------------------- `expect_true(is.<t>(x))` can be hard to read.
          |
          = help: Use `expect_type(x, t)` instead.
        Found 1 error.
        "
        );
        // yoda test with clear expect_type replacement
        assert_snapshot!(
            snapshot_lint("expect_equal('integer', typeof(x))"),
            @r"
        warning: expect_type
         --> <test>:1:1
          |
        1 | expect_equal('integer', typeof(x))
          | ---------------------------------- `expect_equal(typeof(x), t)` can be hard to read.
          |
          = help: Use `expect_type(x, t)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(typeof(x), 'double')",
                    "testthat::expect_identical(typeof(x), 'language')",
                    "expect_true(is.complex(foo(x)))",
                    "expect_equal('integer', typeof(x))",
                    "expect_true(is.character(y))",
                    "testthat::expect_equal(typeof(z), 'list')",
                ],
                "expect_type",
                None
            )
        );
    }

    #[test]
    fn test_expect_type_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("expect_equal(typeof(x), # comment\n'integer')"),
            @r"
        warning: expect_type
         --> <test>:1:1
          |
        1 | / expect_equal(typeof(x), # comment
        2 | | 'integer')
          | |__________- `expect_equal(typeof(x), t)` can be hard to read.
          |
          = help: Use `expect_type(x, t)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(typeof(x), 'double')",
                    "expect_equal(typeof(x), # comment\n'integer')",
                    "expect_true(is.character(x)) # trailing comment",
                ],
                "expect_type",
                None
            )
        );
    }
}
