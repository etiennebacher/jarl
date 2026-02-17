pub(crate) mod expect_match;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_expect_match() {
        expect_no_lint("grepl('Testing is fun', 'fun')", "expect_match", None);
        expect_no_lint(
            "expect_true(grep('Testing is fun', 'fun'))",
            "expect_match",
            None,
        );
        expect_no_lint(
            "expect_false(grepl('Testing is fun', 'fun'))",
            "expect_match",
            None,
        );
        expect_no_lint("expect_true(is.na(x))", "expect_match", None);
        expect_no_lint("expect_true(grepl())", "expect_match", None);
        expect_no_lint("expect_true(grepl(pattern = 'x'))", "expect_match", None);
        expect_no_lint("expect_true(grepl(x = 'y'))", "expect_match", None);
    }

    #[test]
    fn test_lint_expect_match() {
        let lint_msg = "`expect_true(grepl(...))` is not as clear as expect_match(...).";

        expect_lint(
            "testthat::expect_true(grepl('Testing is fun', 'fun'))",
            lint_msg,
            "expect_match",
            None,
        );
        expect_lint(
            "show_failure(expect_true(grepl('Testing is fun', 'fun')))",
            lint_msg,
            "expect_match",
            None,
        );
        expect_lint(
            "expect_true(grepl('Testing is fun', 'fun'), info = 'msg')",
            lint_msg,
            "expect_match",
            None,
        );
        expect_lint(
            "expect_true(grepl('Testing is fun', 'fun'), label = 'lbl')",
            lint_msg,
            "expect_match",
            None,
        );
        expect_lint(
            "expect_true(grepl(pattern = 'Testing is fun', x = 'fun'))",
            lint_msg,
            "expect_match",
            None,
        );
        expect_lint(
            "expect_true(base::grepl('Testing is fun', 'fun'))",
            lint_msg,
            "expect_match",
            None,
        );
    }

    #[test]
    fn test_fix_expect_match() {
        use insta::assert_snapshot;

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_true(grepl('Testing is fun', 'fun'))",
                    "testthat::expect_true(grepl('Testing is fun', 'fun'))",
                    "expect_true(grepl(pattern = 'Testing is fun', x = 'fun'))",
                    "expect_true(grepl('Testing is fun', 'fun', perl = TRUE, fixed = FALSE))",
                ],
                "expect_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_match_with_comments_no_fix() {
        use insta::assert_snapshot;

        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "expect_true(grepl(# comment\n'Testing is fun', 'fun'))",
                    "expect_true(grepl('Testing is fun', # comment\n'fun'))",
                    "expect_true(grepl('Testing is fun', 'fun', # comment\nperl = TRUE))",
                ],
                "expect_match",
                None,
            )
        );
    }
}
