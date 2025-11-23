pub(crate) mod browser;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_browser() {
        expect_no_lint("# browser()", "browser", None);
        expect_no_lint("function(browser = 'firefox')", "browser", None);
        expect_no_lint("function(tool = browser)", "browser", None);
    }

    #[test]
    fn test_lint_browser() {
        let expected_message = "Calls to `browser()` should be removed.";
        expect_lint("browser()", expected_message, "browser", None);
        expect_lint(
            "browser(text = 'remove before commit')",
            expected_message,
            "browser",
            None,
        );
        expect_lint(
            "if (x > 10) { browser(text = 'x is large') }",
            expected_message,
            "browser",
            None,
        );
        expect_lint(
            // This is invalid syntax (invalid 'y' type in 'x || y'), but it "works" for debugging
            "( x < 10 ) || browser('big x')",
            expected_message,
            "browser",
            None,
        );
    }
}
