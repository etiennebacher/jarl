pub(crate) mod expect_type;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

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
        use insta::assert_snapshot;

        expect_lint(
            "expect_equal(typeof(x), 'double')",
            "`expect_equal(typeof(x), t)` can be hard to read",
            "expect_type",
            None,
        );

        // expect_identical is treated the same as expect_equal
        expect_lint(
            "testthat::expect_identical(typeof(x), 'language')",
            "`expect_identical(typeof(x), t)` can be hard to read",
            "expect_type",
            None,
        );

        // different equivalent usage
        expect_lint(
            "expect_true(is.complex(foo(x)))",
            "`expect_true(is.<t>(x))` can be hard to read",
            "expect_type",
            None,
        );

        // yoda test with clear expect_type replacement
        expect_lint(
            "expect_equal('integer', typeof(x))",
            "`expect_equal(typeof(x), t)` can be hard to read",
            "expect_type",
            None,
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
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "expect_equal(typeof(x), # comment\n'integer')",
            "`expect_equal(typeof(x), t)` can be hard to read",
            "expect_type",
            None,
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
