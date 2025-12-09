pub(crate) mod expect_s3_class;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_expect_s3_class() {
        // We don't handle those args
        expect_no_lint(
            "expect_equal(class(x), 'a', info = 'x should have class k')",
            "expect_s3_class",
            None,
        );
        expect_no_lint(
            "expect_equal(class(x), 'a', label = 'x class')",
            "expect_s3_class",
            None,
        );
        expect_no_lint(
            "expect_equal(class(x), 'a', expected.label = 'target class')",
            "expect_s3_class",
            None,
        );

        // Those do not work in `expect_s3_class()`.
        expect_no_lint("expect_equal(class(x), 'list')", "expect_s3_class", None);
        expect_no_lint("expect_equal(class(x), 'logical')", "expect_s3_class", None);
        expect_no_lint("expect_equal(class(x), 'matrix')", "expect_s3_class", None);

        // Not sure if those should be fixed here because if it's an object then
        // it could contain classes that don't work in `expect_s3_class()`.
        expect_no_lint("expect_equal(class(x), k)", "expect_s3_class", None);
        expect_no_lint(
            "expect_equal(class(x), c('a', 'b')",
            "expect_s3_class",
            None,
        );

        // Wrong code but no panic
        expect_no_lint("expect_equal(class(x))", "expect_s3_class", None);
        expect_no_lint("expect_equal(class())", "expect_s3_class", None);
        expect_no_lint(
            "expect_equal(object =, expected =)",
            "expect_s3_class",
            None,
        );
    }

    #[test]
    fn test_lint_expect_s3_class() {
        use insta::assert_snapshot;
        let lint_msg = "may fail if `x` gets more classes in the future";

        expect_lint(
            "expect_equal(class(x), 'data.frame')",
            lint_msg,
            "expect_s3_class",
            None,
        );
        expect_lint(
            "expect_equal(class(x), \"data.frame\")",
            lint_msg,
            "expect_s3_class",
            None,
        );
        expect_lint(
            "testthat::expect_equal(class(x), 'data.frame')",
            lint_msg,
            "expect_s3_class",
            None,
        );
        expect_lint(
            "expect_equal('data.frame', class(x))",
            lint_msg,
            "expect_s3_class",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(class(x), 'data.frame')",
                    "expect_equal(class(x), \"data.frame\")",
                    "testthat::expect_equal(class(x), 'data.frame')",
                    "expect_equal('data.frame', class(x))",
                ],
                "expect_s3_class",
                None,
            )
        );
    }

    #[test]
    fn test_expect_s3_class_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present
        expect_lint(
            "expect_equal(class(x),\n # a comment \n'data.frame')",
            "may fail if `x` gets more classes in the future",
            "expect_s3_class",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(class(x), 'data.frame')",
                    "expect_equal(class(x),\n # a comment \n'data.frame')",
                    "expect_equal(class(x), 'data.frame') # trailing comment",
                ],
                "expect_s3_class",
                None
            )
        );
    }
}
