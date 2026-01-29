pub(crate) mod list2df;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_list2df() {
        expect_no_lint("cbind.data.frame(x, x)", "list2df", Some("4.0"));
        expect_no_lint("do.call(mean, x)", "list2df", Some("4.0"));
        expect_no_lint("do.call('c', x)", "list2df", Some("4.0"));
        expect_no_lint("do.call(cbind, x)", "list2df", Some("4.0"));
        expect_no_lint("do.call(function(x) x, l)", "list2df", Some("4.0"));
        // Ignored if R version unknown or below 4.0.0
        expect_no_lint("do.call(cbind.data.frame, x)", "list2df", Some("3.5"));
        expect_no_lint("do.call(cbind.data.frame, x)", "list2df", None);

        // Don't know how to handle additional comments
        expect_no_lint(
            "do.call(cbind.data.frame, x, quote = TRUE)",
            "list2df",
            Some("4.0"),
        );

        // Ensure that wrong calls are not reported
        expect_no_lint("do.call(cbind.data.frame)", "list2df", Some("4.0"));
        expect_no_lint("do.call(cbind.data.frame, args =)", "list2df", Some("4.0"));
        expect_no_lint("do.call(what =, x)", "list2df", Some("4.0"));
    }

    #[test]
    fn test_lint_list2df() {
        use insta::assert_snapshot;

        let expected_message = "Use `list2DF(x)` instead";
        expect_lint(
            "do.call(cbind.data.frame, x)",
            expected_message,
            "list2df",
            Some("4.0"),
        );
        expect_lint(
            "do.call(args = x, what = cbind.data.frame)",
            expected_message,
            "list2df",
            Some("4.0"),
        );
        expect_lint(
            "do.call(cbind.data.frame, args = x)",
            expected_message,
            "list2df",
            Some("4.0"),
        );
        expect_lint(
            "do.call(cbind.data.frame, foo(bar(x)))",
            expected_message,
            "list2df",
            Some("4.0"),
        );

        // Quoted function names
        expect_lint(
            "do.call('cbind.data.frame', x)",
            expected_message,
            "list2df",
            Some("4.0"),
        );
        expect_lint(
            "do.call(\"cbind.data.frame\", x)",
            expected_message,
            "list2df",
            Some("4.0"),
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "do.call(cbind.data.frame, x)",
                    "do.call('cbind.data.frame', x)",
                    "do.call(\"cbind.data.frame\", x)",
                    "do.call(args = x, what = cbind.data.frame)",
                    "do.call(cbind.data.frame, args = x)",
                    "do.call(cbind.data.frame, foo(bar(x)))",
                ],
                "list2df",
                Some("4.0")
            )
        );
    }

    #[test]
    fn test_list2df_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "do.call(\n # a comment\ncbind.data.frame, x)",
            "Use `list2DF(x)` instead",
            "list2df",
            Some("4.0"),
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\ndo.call(cbind.data.frame, x)",
                    "do.call(\n # a comment\ncbind.data.frame, x)",
                    "do.call(cbind.data.frame, x) # trailing comment",
                ],
                "list2df",
                Some("4.0")
            )
        );
    }
}
