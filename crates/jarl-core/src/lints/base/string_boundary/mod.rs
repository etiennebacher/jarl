pub(crate) mod string_boundary;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "string_boundary", None)
    }

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
        assert_snapshot!(
            snapshot_lint("substr(x, 1, 2) == 'ab'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substr(x, 1, 2) == 'ab'
          | ----------------------- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("substr(x, 1L, 2L) == 'ab'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substr(x, 1L, 2L) == 'ab'
          | ------------------------- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
        );
        // end doesn't matter, just anchoring to 1L
        assert_snapshot!(
            snapshot_lint("substr(x, 1L, end) == 'ab'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substr(x, 1L, end) == 'ab'
          | -------------------------- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
        );
        // != operator also works
        assert_snapshot!(
            snapshot_lint("substr(x, 1L, end) != 'ab'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substr(x, 1L, end) != 'ab'
          | -------------------------- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("substr(x, 3, nchar(x)) != 'ab'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substr(x, 3, nchar(x)) != 'ab'
          | ------------------------------ Using `substr()` to detect a terminal substring is hard to read and inefficient.
          |
          = help: Use `endsWith()` instead.
        Found 1 error.
        "
        );
        // Works in the other direction
        assert_snapshot!(
            snapshot_lint("'ab' == substr(x, 1L, end)"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | 'ab' == substr(x, 1L, end)
          | -------------------------- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(

            snapshot_lint("substring(x, nchar(x) - 4L, nchar(x)) == 'abcde'"),

            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substring(x, nchar(x) - 4L, nchar(x)) == 'abcde'
          | ------------------------------------------------ Using `substring()` to detect a terminal substring is hard to read and inefficient.
          |
          = help: Use `endsWith()` instead.
        Found 1 error.
        "
        );
        // start doesn't matter, just anchoring to nchar(x)
        assert_snapshot!(
            snapshot_lint("substring(x, start, nchar(x)) == 'abcde'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substring(x, start, nchar(x)) == 'abcde'
          | ---------------------------------------- Using `substring()` to detect a terminal substring is hard to read and inefficient.
          |
          = help: Use `endsWith()` instead.
        Found 1 error.
        "
        );
        // more complicated expressions
        assert_snapshot!(
            snapshot_lint("substring(colnames(x), start, nchar(colnames(x))) == 'abc'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substring(colnames(x), start, nchar(colnames(x))) == 'abc'
          | ---------------------------------------------------------- Using `substring()` to detect a terminal substring is hard to read and inefficient.
          |
          = help: Use `endsWith()` instead.
        Found 1 error.
        "
        );
        // comparing vectors
        assert_snapshot!(
            snapshot_lint("substr(c('abc', 'def'), 1, 1) == c('a', 'a')"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | substr(c('abc', 'def'), 1, 1) == c('a', 'a')
          | -------------------------------------------- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
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
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("substr(x, \n # a comment \n1, 2) == 'ab'"),
            @r"
        warning: string_boundary
         --> <test>:1:1
          |
        1 | / substr(x, 
        2 | |  # a comment 
        3 | | 1, 2) == 'ab'
          | |_____________- Using `substr()` to detect an initial substring is hard to read and inefficient.
          |
          = help: Use `startsWith()` instead.
        Found 1 error.
        "
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
