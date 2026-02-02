pub(crate) mod for_loop_index;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_for_loop_index() {
        expect_no_lint("for (xi in x) {}", "for_loop_index", None);
        expect_no_lint("for (col in DF$col) {}", "for_loop_index", None);
        expect_no_lint("for (col in S4@col) {}", "for_loop_index", None);
        expect_no_lint("for (col in DT[, col]) {}", "for_loop_index", None);
        expect_no_lint(
            "{
        for (i in 1:10) {
          42L
        }
        i <- 7L
      }",
            "for_loop_index",
            None,
        );
    }

    #[test]
    fn test_lint_for_loop_index() {
        use insta::assert_snapshot;

        let expected_message = "Don't re-use any sequence symbols as the index";
        expect_lint("for (x in x) {}", expected_message, "for_loop_index", None);
        expect_lint(
            "for (x in foo(x)) {}",
            expected_message,
            "for_loop_index",
            None,
        );
        expect_lint(
            "for (x in foo(x = 1)) {}",
            expected_message,
            "for_loop_index",
            None,
        );
        expect_lint(
            "for (x in foo(bar(y, baz(2, x)))) {}",
            expected_message,
            "for_loop_index",
            None,
        );
        expect_lint(
            "for (x in foo(bar(y, baz(2, x = z)))) {}",
            expected_message,
            "for_loop_index",
            None,
        );

        // No fixes
        assert_snapshot!(
            "fix_output",
            get_fixed_text(vec!["for (x in x) {}",], "for_loop_index", None)
        );
    }

    #[test]
    fn test_for_loop_index_diagnostic_ranges() {
        use crate::utils_test::expect_diagnostic_highlight;

        expect_diagnostic_highlight(
            "for (x in foo(x)) { TRUE }",
            "for_loop_index",
            "x in foo(x)",
        );
        expect_diagnostic_highlight(
            "for (x in foo(\nx\n)) { TRUE }",
            "for_loop_index",
            "x in foo(\nx\n)",
        );
    }
}
