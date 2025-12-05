pub(crate) mod fixed_regex;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_fixed_regex() {
        // Patterns with regex special characters
        expect_no_lint("gsub('^x', '', y)", "fixed_regex", None);
        expect_no_lint("grep('x$', y)", "fixed_regex", None);
        expect_no_lint("grepv('x$', y)", "fixed_regex", None);
        expect_no_lint("sub('[a-zA-Z]', '', y)", "fixed_regex", None);
        expect_no_lint("{regexec('\\s', '', y)}", "fixed_regex", None);
        expect_no_lint("grep('a(?=b)', x, perl = TRUE)", "fixed_regex", None);
        expect_no_lint("grep('0+1', x, perl = TRUE)", "fixed_regex", None);
        expect_no_lint("grep('1*2', x)", "fixed_regex", None);
        expect_no_lint("grep('a|b', x)", "fixed_regex", None);
        expect_no_lint("{grep('\\[|\\]', x)}", "fixed_regex", None);

        // Pattern is not a string literal
        expect_no_lint("grepl(fmt, y)", "fixed_regex", None);

        // fixed = TRUE is already set, regex patterns don't matter
        expect_no_lint("{gsub('abc', '', y, fixed = TRUE)}", "fixed_regex", None);

        // TODO: once again, get_arg_by_name_then_position() fails to get the correct value
        // fixed = TRUE but by position
        // expect_no_lint(
        //     "{gsub('abc', '', y, ignore.case = FALSE, perl = FALSE, TRUE)}",
        //     "fixed_regex",
        //     None,
        // );

        // ignore.case=TRUE implies regex interpretation
        expect_no_lint(
            "gsub('abcdefg', '', y, ignore.case = TRUE)",
            "fixed_regex",
            None,
        );

        // char classes starting with [] might contain other characters -> not fixed
        expect_no_lint("sub('[][]', '', y)", "fixed_regex", None);
        expect_no_lint("sub('[][ ]', '', y)", "fixed_regex", None);
        expect_no_lint("sub('[],[]', '', y)", "fixed_regex", None);

        // wrapper functions don't throw
        expect_no_lint(
            "gregexpr(pattern = pattern, data, perl = TRUE, ...)",
            "fixed_regex",
            None,
        );
    }

    #[test]
    fn test_lint_fixed_regex() {
        use insta::assert_snapshot;
        let lint_msg = "Add `fixed = TRUE` for better performance";

        expect_lint("grepl('abcdefg', x)", lint_msg, "fixed_regex", None);
        expect_lint("grep('abcdefg', x)", lint_msg, "fixed_regex", None);
        expect_lint("regexec('abcdefg', x)", lint_msg, "fixed_regex", None);
        expect_lint("regexpr('abcdefg', x)", lint_msg, "fixed_regex", None);
        expect_lint("gsub('abcdefg', 'a', x)", lint_msg, "fixed_regex", None);
        expect_lint("sub('abcdefg', 'a', x)", lint_msg, "fixed_regex", None);
        expect_lint("gregexpr('abcdefg', x)", lint_msg, "fixed_regex", None);

        expect_lint("gregexpr('a-z', y)", lint_msg, "fixed_regex", None);

        // naming the argument doesn't matter (if it's still used positionally)
        expect_lint(
            "gregexpr(pattern = 'a-z', y)",
            lint_msg,
            "fixed_regex",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "grepl('abcdefg', x)",
                    "grep('abcdefg', x)",
                    "regexec('abcdefg', x)",
                    "regexpr('abcdefg', x)",
                    "gsub('abcdefg', 'a', x)",
                    "sub('abcdefg', 'a', x)",
                    "gregexpr('abcdefg', x)",
                    "gregexpr('a-z', y)",
                    "gregexpr('a-z', y, fixed = FALSE)",
                    "gregexpr('a-z', y, fixed = FALSE, ignore.case = FALSE)",
                    "gregexpr(pattern = 'a-z', y)",
                ],
                "fixed_regex",
                None
            )
        );
    }

    #[test]
    fn test_fixed_regex_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "grep(\n  # comment\n  'hello', x\n)",
            "Add `fixed = TRUE` for better performance",
            "fixed_regex",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\ngrep('hello', x)",
                    "grep(\n  # comment\n  'hello', x\n)",
                    "grep('hello',\n    # comment\n    x)",
                    "grep('hello', x) # trailing comment",
                ],
                "fixed_regex",
                None
            )
        );
    }
}
