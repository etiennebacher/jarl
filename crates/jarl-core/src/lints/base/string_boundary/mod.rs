pub(crate) mod string_boundary;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_string_boundary() {
        // no comparison operator --> no lint
        expect_no_lint("substr(x, start, end)", "string_boundary", None);
        // unknown indices --> no lint
        expect_no_lint("substr(x, start, end) == 'a'", "string_boundary", None);
        expect_no_lint("substring(x, start, end) == 'a'", "string_boundary", None);
        // using foo(nchar(.))
        expect_no_lint(
            "substring(x, nchar(x) - 4, nchar(x) - 1) == 'abc'",
            "string_boundary",
            None,
        );
        // using nchar(), but not of the input
        expect_no_lint(
            "substring(x, nchar(y) - 4, nchar(y)) == 'abcd'",
            "string_boundary",
            None,
        );
        // using x in nchar(), but on foo(input)
        expect_no_lint(
            "substring(x, nchar(foo(x)) - 4, nchar(foo(x))) == 'abcd'",
            "string_boundary",
            None,
        );
        // Unknown function in stop
        expect_no_lint("substring(x, 2, foo(x)) == 'abcd'", "string_boundary", None);
        // Wrong nchar() call
        expect_no_lint(
            "substring(x, 2, nchar(x, y)) == 'abcd'",
            "string_boundary",
            None,
        );
        expect_no_lint(
            "substring(x, 2, nchar(x,)) == 'abcd'",
            "string_boundary",
            None,
        );
        // Unknown object in `stop`
        expect_no_lint("substring(x, 2, y) == 'abcd'", "string_boundary", None);

        // _close_ to equivalent, but not so in general -- e.g.
        //   substring(s <- "abcdefg", 2L) == "efg" is not TRUE, but endsWith(s, "efg")
        //   is. And if `s` contains strings of varying lengths, there's no equivalent.
        expect_no_lint("substring(x, 2L)", "string_boundary", None);
    }

    #[test]
    fn test_lint_string_boundary() {
        use insta::assert_snapshot;

        expect_lint(
            "substr(x, 1, 2) == 'ab'",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );
        expect_lint(
            "substr(x, 1L, 2L) == 'ab'",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );
        // end doesn't matter, just anchoring to 1L
        expect_lint(
            "substr(x, 1L, end) == 'ab'",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );
        // != operator also works
        expect_lint(
            "substr(x, 1L, end) != 'ab'",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );
        expect_lint(
            "substr(x, 3, nchar(x)) != 'ab'",
            "Using `substr()` to detect a terminal substring",
            "string_boundary",
            None,
        );
        // Works in the other direction
        expect_lint(
            "'ab' == substr(x, 1L, end)",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );

        expect_lint(
            "substring(x, nchar(x) - 4L, nchar(x)) == 'abcde'",
            "Using `substring()` to detect a terminal substring",
            "string_boundary",
            None,
        );
        // start doesn't matter, just anchoring to nchar(x)
        expect_lint(
            "substring(x, start, nchar(x)) == 'abcde'",
            "Using `substring()` to detect a terminal substring",
            "string_boundary",
            None,
        );
        // more complicated expressions
        expect_lint(
            "substring(colnames(x), start, nchar(colnames(x))) == 'abc'",
            "Using `substring()` to detect a terminal substring",
            "string_boundary",
            None,
        );
        // comparing vectors
        expect_lint(
            "substr(c('abc', 'def'), 1, 1) == c('a', 'a')",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "substr(x, 1, 2) == 'ab'",
                    "substr(x, 1L, 2L) == 'ab'",
                    "substr(x, 1L, end) == 'ab'",
                    "substr(x, 1L, end) != 'ab'",
                    "substr(x, 3, nchar(x)) != 'ab'",
                    "'ab' == substr(x, 1L, end)",
                    "substring(x, nchar(x) - 4L, nchar(x)) == 'abcde'",
                    "substring(x, start, nchar(x)) == 'abcde'",
                    "substring(colnames(x), start, nchar(colnames(x))) == 'abc'",
                    "substr(c('abc', 'def'), 1, 1) == c('a', 'a')",
                ],
                "string_boundary",
                None
            )
        );
    }

    #[test]
    fn test_string_boundary_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "substr(x, \n # a comment \n1, 2) == 'ab'",
            "Using `substr()` to detect an initial substring",
            "string_boundary",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsubstr(x, 1, 2) == 'ab'",
                    "substr(x, \n # a comment \n1, 2) == 'ab'",
                    "substr(x, 1, 2) == 'ab' # trailing comment",
                ],
                "string_boundary",
                None
            )
        );
    }
}
