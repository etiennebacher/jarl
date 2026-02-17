pub(crate) mod assignment;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::assignment::ResolvedAssignmentOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use air_r_syntax::RSyntaxKind;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "assignment", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "assignment", None, Some(settings))
    }

    /// Build a `Settings` with a specific assignment operator.
    fn settings_with_options(operator: RSyntaxKind) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    assignment: ResolvedAssignmentOptions { operator },
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_lint_assignment() {
        assert_snapshot!(
            snapshot_lint("blah=1"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | blah=1
          | ----- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("blah = 1"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | blah = 1
          | ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("blah = fun(1)"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | blah = fun(1)
          | ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("names(blah) = 'a'"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | names(blah) = 'a'
          | ------------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[[1]] = 2"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | x[[1]] = 2
          | -------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("fun((blah = fun(1)))"),
            @r"
        warning: assignment
         --> <test>:1:6
          |
        1 | fun((blah = fun(1)))
          |      ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1 -> fun"),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 1 -> fun
          |   ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1 -> names(fun)"),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 1 -> names(fun)
          |   ------------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("2 -> x[[1]]"),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 2 -> x[[1]]
          |   --------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "blah=1",
                    "blah = 1",
                    "blah = fun(1)",
                    "names(blah) = 'a'",
                    "x[[1]] = 2",
                    "fun((blah = fun(1)))",
                    "1 -> fun",
                    "'a' -> names(fun)",
                    "2 -> x[[1]]",
                ],
                "assignment",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_assignment() {
        expect_no_lint("y <- 1", "assignment", None);
        expect_no_lint("fun(y = 1)", "assignment", None);
        expect_no_lint("y == 1", "assignment", None);
    }

    #[test]
    fn test_assignment_diagnostic_ranges() {
        use crate::utils_test::expect_diagnostic_highlight;

        expect_diagnostic_highlight("x = 1", "assignment", "x =");
        expect_diagnostic_highlight("x=1", "assignment", "x=");
        expect_diagnostic_highlight("1 -> x", "assignment", "-> x");
        expect_diagnostic_highlight("foo() |>\n  bar() |>\n  baz() -> x", "assignment", "-> x");
        // TODO: uncomment when https://github.com/etiennebacher/jarl/issues/89 is fixed
        // expect_diagnostic_highlight("1 -> names(\nx)", "assignment", "-> names(\nx)");
    }

    // ---- Rule-specific config tests (operator = "=") ----

    #[test]
    fn test_lint_assignment_with_equal_operator() {
        let settings = settings_with_options(RSyntaxKind::EQUAL);

        // `y <- 1` should lint when operator = "="
        assert_snapshot!(
            snapshot_lint_with_settings("y <- 1", settings.clone()),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | y <- 1
          | ---- Use `=` for assignment.
          |
        Found 1 error.
        "
        );

        // `1 -> z` should lint when operator = "="
        assert_snapshot!(
            snapshot_lint_with_settings("1 -> z", settings),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 1 -> z
          |   ---- Use `=` for assignment.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_assignment_with_equal_operator() {
        let settings = settings_with_options(RSyntaxKind::EQUAL);

        expect_no_lint_with_settings("x = 1", "assignment", None, settings.clone());
        expect_no_lint_with_settings("fun(y = 1)", "assignment", None, settings.clone());
        expect_no_lint_with_settings("y == 1", "assignment", None, settings);
    }

    #[test]
    fn test_lint_assignment_default_operator() {
        // Default operator is ASSIGN (<-), so `x = 1` should lint
        let settings = settings_with_options(RSyntaxKind::ASSIGN);

        assert_snapshot!(
            snapshot_lint_with_settings("x = 1", settings.clone()),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | x = 1
          | --- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );

        // `y <- 1` should NOT lint with default operator
        expect_no_lint_with_settings("y <- 1", "assignment", None, settings);
    }
}
