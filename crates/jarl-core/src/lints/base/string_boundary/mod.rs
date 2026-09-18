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

        for code in [
            "substr(x, 1, 2) == 'a'",
            "substr(x, 1, 1) == 'ab'",
            "substr(x, 1, 2) != 'a'",
            "'a' == substr(x, 1, 2)",
            "'a' != substr(x, 1, 2)",
            "substr(x, 1, nchar(x)) == 'a'",
            "substr(x, 1L, end) == 'ab'",
            "substr(x, 1L, end) != 'ab'",
            "'ab' == substr(x, 1L, end)",
            "substr(x, 3, nchar(x)) != 'ab'",
            "substring(x, start, nchar(x)) == 'abcde'",
            "substring(colnames(x), start, nchar(colnames(x))) == 'abc'",
            "substring(x, nchar(x) - 1, nchar(x)) == 'b'",
            "substring(x, nchar(x) - 1, nchar(x)) != 'b'",
            "substr(x, nchar(x), nchar(x)) == 'ab'",
            "substring(x, nchar(x) - 2, nchar(x)) == 'ab'",
            "substring(x, nchar(y) - 1, nchar(x)) == 'ab'",
            "substring(x, nchar(x) + 1, nchar(x)) == 'ab'",
            "substring(x, nchar(x) - offset, nchar(x)) == 'ab'",
            "substring(x, nchar(x, type = 'bytes') - 1, nchar(x)) == 'ab'",
            "substring(x, nchar(x) - 1, nchar(x, type = 'bytes')) == 'ab'",
            "substring(x, nchar(allowNA = x) - 1, nchar(allowNA = x)) == 'ab'",
            "substr(x, 1, 1) == ''",
            "substr(x, 1, 1) == r\"()\"",
            "substr(x, 1, 1) == pattern",
            "substr(x, 1, 1) == 1",
            "substr(c('abc', 'def'), 1, 1) == c('a', 'a')",
            "substr(x, 1, 3) == '你'",
            "substring(x, nchar(x) - 2, nchar(x)) == '你'",
            r#"substr(x, 1, 6) == "\u4f60""#,
            r#"substr(x, 1, 1) == "\u4f60""#,
            r#"substr(x, 1, 2) == "\n""#,
            r#"substr(x, 1, 1) == "\n""#,
            r#"substr(x, 1, 3) == r"(你)""#,
            "substr(x, 1, 0) == 'a'",
            "substr(x, 1, -1) == 'a'",
            "substr(x, 1, 1.5) == 'a'",
            "substr(x, 1, 1e100) == 'a'",
            "substr(x, 1, 0x2) == 'ab'",
            "substr(x, 1, 2147483648L) == 'a'",
            "substr(x, 1, 9999999999999999999999999999999999999999) == 'a'",
            "substr(x, stop = 1, start = 2) == 'ab'",
            "substring(x, last = 1, first = 2) == 'ab'",
            "substr(x, 1, 2, extra = 3) == 'ab'",
            "substr(x, 1, 2,) == 'ab'",
            "substr(x, 1,) == 'ab'",
            "substr(, 1, 2) == 'ab'",
            "substr(x, start = 1, start = 2) == 'ab'",
            "substr(x, first = 1, stop = 2) == 'ab'",
            "substr(x, sta = 1, sto = 2) == 'ab'",
        ] {
            expect_no_lint(code, "string_boundary", None);
        }
    }

    #[test]
    fn test_lint_string_boundary() {
        assert_snapshot!(
            snapshot_lint("substr(x, 1, 2) == 'ab'"),
            @"
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
            @"
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
        assert_snapshot!(
            snapshot_lint("substring(x, nchar(x) - 4L, nchar(x)) == 'abcde'"),
            @"
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
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "substr(x, 1, 2) == 'ab'",
                    "substr(x, 1L, 2L) == 'ab'",
                    "substr(x, 1.0, 2.0) == 'ab'",
                    "substr(x, 1., 2e0) == 'ab'",
                    "substr(x, 1L, 2L) != 'ab'",
                    "'ab' == substr(x, 1L, 2L)",
                    "'ab' != substr(x, 1L, 2L)",
                    "substring(x, 1, 2) == 'ab'",
                    "substr(stop = 2, x = x, start = 1) == 'ab'",
                    "substr(stop = 2, x, 1) == 'ab'",
                    "substring(last = 2, text = x, first = 1) == 'ab'",
                    "substring(x, nchar(x) - 4L, nchar(x)) == 'abcde'",
                    "substr(x, nchar(x) - 1, nchar(x)) != 'ab'",
                    "'ab' == substring(x, nchar(x) - 1, nchar(x))",
                    "'ab' != substring(x, nchar(x) - 1, nchar(x))",
                    "substr(x, nchar(x), nchar(x)) == 'a'",
                    "substring(x, nchar(x) - 0L, nchar(x)) == 'a'",
                    "substring(x, nchar(x = x) - 1, nchar(x = x)) == 'ab'",
                    "substring(colnames(x), nchar(colnames(x)) - 2, nchar(colnames(x))) == 'abc'",
                    "substr(x, 1, 1) == '你'",
                    "substring(x, nchar(x) - 1, nchar(x)) == '你好'",
                    "substr(x, 1, 2) == 'a '",
                    "substr(x, 1, 1) == ' '",
                    r#"substr(x, 1, 1) == r"(你)""#,
                    r#"substring(x, nchar(x), nchar(x)) == R'---[你]---'"#,
                    r#"substr(x, 1, 2) == r"(\n)""#,
                ],
                "string_boundary"
            )
        );
    }

    #[test]
    fn test_string_boundary_requires_unsafe_fixes() {
        assert_snapshot!(
            "safe_fix_output",
            get_fixed_text(
                vec![
                    "substr(x, 1, 2) == 'ab'",
                    "substring(x, nchar(x) - 1, nchar(x)) == 'ab'",
                ],
                "string_boundary",
                None
            )
        );
    }

    #[test]
    fn test_string_boundary_mismatched_width_no_fix() {
        assert_snapshot!(
            "no_fix_mismatched_width",
            get_unsafe_fixed_text(
                vec![
                    "substr(x, 1, 2) == 'a'",
                    "substr(x, 1, 1) != 'ab'",
                    "substring(x, nchar(x) - 1, nchar(x)) == 'b'",
                    "substr(x, nchar(x), nchar(x)) == 'ab'",
                    "substr(x, 1, end) == 'ab'",
                    "substring(x, start, nchar(x)) == 'ab'",
                    "substr(x, stop = 1, start = 2) == 'ab'",
                    r#"substr(x, 1, 6) == "\u4f60""#,
                ],
                "string_boundary"
            )
        );
    }

    #[test]
    fn test_string_boundary_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("substr(x, \n # a comment \n1, 2) == 'ab'"),
            @"
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
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nsubstr(x, 1, 2) == 'ab'",
                    "substr(x, \n # a comment \n1, 2) == 'ab'",
                    "substr(x, 1, 2) == 'ab' # trailing comment",
                ],
                "string_boundary"
            )
        );
    }
}
