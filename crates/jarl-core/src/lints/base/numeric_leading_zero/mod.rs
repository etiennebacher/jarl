pub(crate) mod numeric_leading_zero;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "numeric_leading_zero", None)
    }

    #[test]
    fn test_lint_numeric_leading_zero() {
        assert_snapshot!(
            snapshot_lint("a <- .1"),
            @r"
        warning: numeric_leading_zero
         --> <test>:1:6
          |
        1 | a <- .1
          |      -- Include the leading zero for fractional numeric constants.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("b <- -.2"),
            @r"
        warning: numeric_leading_zero
         --> <test>:1:7
          |
        1 | b <- -.2
          |       -- Include the leading zero for fractional numeric constants.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("c <- .3 + 4.5i"),
            @r"
        warning: numeric_leading_zero
         --> <test>:1:6
          |
        1 | c <- .3 + 4.5i
          |      -- Include the leading zero for fractional numeric constants.
          |
        Found 1 error.
        "
        );
        // TODO: uncomment when tree-sitter bug is fixed
        // https://github.com/r-lib/tree-sitter-r/issues/190
        // assert_snapshot!(snapshot_lint("d <- 6.7 + .8i"), @"");
        // assert_snapshot!(snapshot_lint("d <- 6.7+.8i"), @"");
        assert_snapshot!(
            snapshot_lint("e <- .9e10"),
            @r"
        warning: numeric_leading_zero
         --> <test>:1:6
          |
        1 | e <- .9e10
          |      ----- Include the leading zero for fractional numeric constants.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "0.1 + .22-0.3-.2",
                    "d <- 6.7 + .8i",
                    ".7i + .2 + .8i",
                    "'some text .7'",
                ],
                "numeric_leading_zero",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_numeric_leading_zero() {
        expect_no_lint("a <- 0.1", "numeric_leading_zero", None);
        expect_no_lint("b <- -0.2", "numeric_leading_zero", None);
        expect_no_lint("c <- 3.0", "numeric_leading_zero", None);
        expect_no_lint("d <- 4L", "numeric_leading_zero", None);
        expect_no_lint("e <- TRUE", "numeric_leading_zero", None);
        expect_no_lint("f <- 0.5e6", "numeric_leading_zero", None);
        expect_no_lint("g <- 0x78", "numeric_leading_zero", None);
        expect_no_lint("h <- 0.9 + 0.1i", "numeric_leading_zero", None);
        expect_no_lint("h <- 0.9+0.1i", "numeric_leading_zero", None);
        expect_no_lint("h <- 0.9 - 0.1i", "numeric_leading_zero", None);
        expect_no_lint("i <- 2L + 3.4i", "numeric_leading_zero", None);
        expect_no_lint("i <- '.1'", "numeric_leading_zero", None);
    }
}
