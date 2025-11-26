pub(crate) mod expect_named;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_expect_named() {
        // colnames(), rownames(), and dimnames() tests are not equivalent
        expect_no_lint("expect_equal(colnames(x), 'a')", "expect_named", None);
        expect_no_lint("expect_equal(rownames(x), 'a')", "expect_named", None);
        expect_no_lint("expect_equal(dimnames(x), 'a')", "expect_named", None);

        expect_no_lint("expect_equal(nrow(x), 4L)", "expect_named", None);
        expect_no_lint("testthat::expect_equal(nrow(x), 4L)", "expect_named", None);

        // only check the first argument. yoda tests in the second argument will be
        //   missed, but there are legitimate uses of names() in argument 2
        expect_no_lint("expect_equal(colnames(x), names(y))", "expect_named", None);

        // more readable than expect_named(x, names(y))
        expect_no_lint("expect_equal(names(x), names(y))", "expect_named", None);

        // Not the functions we're looking for
        expect_no_lint("expect_equal(x, 'a')", "expect_named", None);
        expect_no_lint("some_other_function(names(x), 'a')", "expect_named", None);

        // Wrong code but no panic
        expect_no_lint("expect_equal(names(x))", "expect_named", None);
        expect_no_lint("expect_equal(names())", "expect_named", None);
        expect_no_lint("expect_equal(object =, expected =)", "expect_named", None);
    }

    #[test]
    fn test_lint_expect_named() {
        use insta::assert_snapshot;
        let lint_msg = "`expect_named(x, n)` is better than";

        expect_lint(
            "expect_equal(names(x), 'a')",
            lint_msg,
            "expect_named",
            None,
        );
        expect_lint(
            "expect_equal(names(x), c('a', 'b'))",
            lint_msg,
            "expect_named",
            None,
        );
        expect_lint(
            "expect_identical(names(x), 'a')",
            lint_msg,
            "expect_named",
            None,
        );
        expect_lint(
            "expect_equal('a', names(x))",
            lint_msg,
            "expect_named",
            None,
        );
        expect_lint(
            "expect_equal(foo(x), names(x))",
            lint_msg,
            "expect_named",
            None,
        );
        expect_lint(
            "expect_equal(names(x), NULL)",
            lint_msg,
            "expect_named",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(names(x), 'a')",
                    "expect_equal(names(x), c('a', 'b'))",
                    "expect_identical(names(x), 'a')",
                    "expect_equal('a', names(x))",
                    "expect_equal(foo(x), names(x))"
                ],
                "expect_named",
                None,
            )
        );
    }

    #[test]
    fn test_expect_named_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present
        expect_lint(
            "expect_equal(# comment\nnames(x), 'a')",
            "`expect_named(x, n)` is better than",
            "expect_named",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(names(x), 'a')",
                    "expect_equal(# comment\nnames(x), 'a')",
                    "expect_equal(names(x), 'a') # trailing comment",
                ],
                "expect_named",
                None
            )
        );
    }
}
