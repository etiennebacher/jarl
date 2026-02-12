pub(crate) mod browser;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "browser", None)
    }

    #[test]
    fn test_no_lint_browser() {
        expect_no_lint("# browser()", "browser", None);
        expect_no_lint("function(browser = 'firefox')", "browser", None);
        expect_no_lint("function(tool = browser)", "browser", None);
    }

    #[test]
    fn test_lint_browser() {
        assert_snapshot!(
            snapshot_lint("browser()"),
            @r"
        warning: browser
         --> <test>:1:1
          |
        1 | browser()
          | --------- Calls to `browser()` should be removed.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("utils::browser()"),
            @r"
        warning: browser
         --> <test>:1:1
          |
        1 | utils::browser()
          | ---------------- Calls to `browser()` should be removed.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("browser(text = 'remove before commit')"),
            @r"
        warning: browser
         --> <test>:1:1
          |
        1 | browser(text = 'remove before commit')
          | -------------------------------------- Calls to `browser()` should be removed.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (x > 10) { browser(text = 'x is large') }"),
            @r"
        warning: browser
         --> <test>:1:15
          |
        1 | if (x > 10) { browser(text = 'x is large') }
          |               ---------------------------- Calls to `browser()` should be removed.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(
            // This is invalid syntax (invalid 'y' type in 'x || y'), but it "works" for debugging
            "( x < 10 ) || browser('big x')"
        ),
        @r"
        warning: browser
         --> <test>:1:15
          |
        1 | ( x < 10 ) || browser('big x')
          |               ---------------- Calls to `browser()` should be removed.
          |
        Found 1 error.
        "
        );
    }
}
