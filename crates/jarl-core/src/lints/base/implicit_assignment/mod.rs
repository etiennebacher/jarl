pub(crate) mod implicit_assignment;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::implicit_assignment::ImplicitAssignmentOptions;
    use crate::rule_options::implicit_assignment::ResolvedImplicitAssignmentOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "implicit_assignment", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "implicit_assignment", None, Some(settings))
    }

    /// Build a `Settings` with custom `ImplicitAssignmentOptions`.
    fn settings_with_options(options: ImplicitAssignmentOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    implicit_assignment: ResolvedImplicitAssignmentOptions::resolve(Some(&options))
                        .unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_lint_implicit_assignment() {
        assert_snapshot!(
            snapshot_lint("if (x <- 1L) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (x <- 1L) TRUE
          |     ------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (1L -> x) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (1L -> x) TRUE
          |     ------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (x <<- 1L) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (x <<- 1L) TRUE
          |     -------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (1L ->> x) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (1L ->> x) TRUE
          |     -------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (A && (B <- foo())) { }"),
            @r"
        warning: implicit_assignment
         --> <test>:1:11
          |
        1 | if (A && (B <- foo())) { }
          |           ---------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (x <- 0L) FALSE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:8
          |
        1 | while (x <- 0L) FALSE
          |        ------- Avoid implicit assignments in `while()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (0L -> x) FALSE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:8
          |
        1 | while (0L -> x) FALSE
          |        ------- Avoid implicit assignments in `while()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in y <- 1:10) print(x)"),
            @r"
        warning: implicit_assignment
         --> <test>:1:11
          |
        1 | for (x in y <- 1:10) print(x)
          |           --------- Avoid implicit assignments in `for()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in 1:10 -> y) print(x)"),
            @r"
        warning: implicit_assignment
         --> <test>:1:11
          |
        1 | for (x in 1:10 -> y) print(x)
          |           --------- Avoid implicit assignments in `for()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(x <- 1 > 2)"),
            @r"
        warning: implicit_assignment
         --> <test>:1:13
          |
        1 | expect_true(x <- 1 > 2)
          |             ---------- Avoid implicit assignments in function calls.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_implicit_assignment() {
        expect_no_lint("x <- 1", "implicit_assignment", None);
        expect_no_lint("x <- { 3 + 4 }", "implicit_assignment", None);
        expect_no_lint("y <- if (is.null(x)) z else x", "implicit_assignment", None);
        expect_no_lint("foo({\na <- 1L\n})", "implicit_assignment", None);
        expect_no_lint("if (1 + 1) TRUE", "implicit_assignment", None);
        expect_no_lint("a %>% b()", "implicit_assignment", None);
        expect_no_lint("a |> b()", "implicit_assignment", None);
        expect_no_lint("if (TRUE) {\nx <- 1\n}", "implicit_assignment", None);
        expect_no_lint("while (TRUE) {\nx <- 1\n}", "implicit_assignment", None);
        expect_no_lint("for (i in 1:2) {\nx <- 1\n}", "implicit_assignment", None);
        expect_no_lint("if (TRUE) x <- 1", "implicit_assignment", None);
        expect_no_lint("for (i in 1:2) x <- 1", "implicit_assignment", None);
        expect_no_lint("while (TRUE) x <- 1", "implicit_assignment", None);
        expect_no_lint(
            "f <- function() {
          if (TRUE)
            x <- 1
          else
            x <- 2
        }
        ",
            "implicit_assignment",
            None,
        );

        // Whitelist: https://github.com/etiennebacher/jarl/issues/133
        expect_no_lint("expect_message(x <- 1)", "implicit_assignment", None);
        expect_no_lint("expect_warning(x <- 1)", "implicit_assignment", None);
        expect_no_lint("expect_error(x <- 1)", "implicit_assignment", None);
        expect_no_lint("expect_snapshot(x <- 1)", "implicit_assignment", None);
        expect_no_lint("quote(x <- 1)", "implicit_assignment", None);
        expect_no_lint("suppressMessages(x <- 1)", "implicit_assignment", None);
        expect_no_lint("suppressWarnings(x <- 1)", "implicit_assignment", None);
        expect_no_lint("suppressWarnings({x <- 1})", "implicit_assignment", None);
    }

    // ---- Rule-specific config tests ----

    #[test]
    fn test_skipped_functions_replaces_defaults() {
        // With custom skipped-functions = ["list"], only "list" is skipped.
        // Default-skipped "expect_error" should now lint.
        let settings = settings_with_options(ImplicitAssignmentOptions {
            skipped_functions: Some(vec!["list".to_string()]),
            extend_skipped_functions: None,
        });

        // "list" is in the custom list -> no lint
        expect_no_lint_with_settings(
            "list(a <- 1)",
            "implicit_assignment",
            None,
            settings.clone(),
        );

        // "expect_error" is NOT in the custom list -> now lints (was default-skipped)
        assert_snapshot!(
            snapshot_lint_with_settings("expect_error(a <- 1)", settings),
            @r"
        warning: implicit_assignment
         --> <test>:1:14
          |
        1 | expect_error(a <- 1)
          |              ------ Avoid implicit assignments in function calls.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_extend_skipped_functions_adds_to_defaults() {
        // extend-skipped-functions = ["my_fun"] -> defaults + "my_fun"
        let settings = settings_with_options(ImplicitAssignmentOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["my_fun".to_string()]),
        });

        // "my_fun" is in the extended list -> no lint
        expect_no_lint_with_settings(
            "my_fun(a <- 1)",
            "implicit_assignment",
            None,
            settings.clone(),
        );

        // Default "expect_error" is still skipped
        expect_no_lint_with_settings(
            "expect_error(a <- 1)",
            "implicit_assignment",
            None,
            settings.clone(),
        );

        // "foo" is not in either list -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("foo(a <- 1)", settings),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | foo(a <- 1)
          |     ------ Avoid implicit assignments in function calls.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_skipped_functions_with_namespaced_call() {
        // implicit_assignment extracts just the RHS of pkg::fun, so
        // skipping "my_fun" also skips "pkg::my_fun(...)".
        let settings = settings_with_options(ImplicitAssignmentOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["my_fun".to_string()]),
        });

        expect_no_lint_with_settings("pkg::my_fun(a <- 1)", "implicit_assignment", None, settings);
    }

    #[test]
    fn test_implicit_assignment_with_interceding_comments() {
        assert_snapshot!(
            snapshot_lint(
            "fun(
                a <- # xxx
                1,
              )"), @r"
        warning: implicit_assignment
         --> <test>:2:17
          |
        2 | /                 a <- # xxx
        3 | |                 1,
          | |_________________- Avoid implicit assignments in function calls.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_namespaced_value_in_config_does_not_match_plain_call() {
        // If the user puts "mypkg::myfun" in the config, only the function name
        // is matched (i.e. "myfun"), so a plain call to `myfun(...)` should NOT
        // match "mypkg::myfun".
        let settings = settings_with_options(ImplicitAssignmentOptions {
            skipped_functions: Some(vec!["mypkg::myfun".to_string()]),
            extend_skipped_functions: None,
        });

        let code = r#"myfun(a <- 1)"#;
        assert_snapshot!(
            snapshot_lint_with_settings(code, settings),
            @r"
        warning: implicit_assignment
         --> <test>:1:7
          |
        1 | myfun(a <- 1)
          |       ------ Avoid implicit assignments in function calls.
          |
        Found 1 error.
        "
        );
    }
}
