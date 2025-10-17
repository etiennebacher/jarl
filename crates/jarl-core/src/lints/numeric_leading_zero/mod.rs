pub(crate) mod numeric_leading_zero;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_numeric_leading_zero() {
        use insta::assert_snapshot;

        let expected_message = "Include the leading zero";
        expect_lint("a <- .1", expected_message, "numeric_leading_zero", None);
        expect_lint("b <- -.2", expected_message, "numeric_leading_zero", None);
        expect_lint(
            "c <- .3 + 4.5i",
            expected_message,
            "numeric_leading_zero",
            None,
        );
        // TODO: uncomment when tree-sitter bug is fixed
        // https://github.com/r-lib/tree-sitter-r/issues/190
        // expect_lint(
        //     "d <- 6.7 + .8i",
        //     expected_message,
        //     "numeric_leading_zero",
        //     None,
        // );
        // expect_lint(
        //     "d <- 6.7+.8i",
        //     expected_message,
        //     "numeric_leading_zero",
        //     None,
        // );
        expect_lint("e <- .9e10", expected_message, "numeric_leading_zero", None);
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
