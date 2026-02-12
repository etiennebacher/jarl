pub(crate) mod duplicated_arguments;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::duplicated_arguments::DuplicatedArgumentsOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "duplicated_arguments", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "duplicated_arguments", None, Some(settings))
    }

    /// Build a `Settings` with custom `DuplicatedArgumentsOptions`.
    fn settings_with_options(options: DuplicatedArgumentsOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions::resolve(Some(&options), None).unwrap(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_no_lint_duplicated_arguments() {
        expect_no_lint("fun(arg = 1)", "duplicated_arguments", None);
        expect_no_lint("fun('arg' = 1)", "duplicated_arguments", None);
        expect_no_lint("fun(`arg` = 1)", "duplicated_arguments", None);
        expect_no_lint("'fun'(arg = 1)", "duplicated_arguments", None);
        expect_no_lint(
            "(function(x, y) x + y)(x = 1)",
            "duplicated_arguments",
            None,
        );
        expect_no_lint(
            "fun(x = (function(x) x + 1), y = 1)",
            "duplicated_arguments",
            None,
        );
        expect_no_lint("dt[i = 1]", "duplicated_arguments", None);
        expect_no_lint(
            "cli_format_each_inline(x = 'a', x = 'a')",
            "duplicated_arguments",
            None,
        );

        // `"` and `'` are not the same argument names.
        expect_no_lint("switch(x, `\"` = 1, `'` = 2)", "duplicated_arguments", None);
    }

    #[test]
    fn test_lint_duplicated_arguments() {
        assert_snapshot!(
            snapshot_lint("fun(arg = 1, arg = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | fun(arg = 1, arg = 2)
          | --------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("fun(arg = 1, 'arg' = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | fun(arg = 1, 'arg' = 2)
          | ----------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("fun(arg = 1, `arg` = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | fun(arg = 1, `arg` = 2)
          | ----------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("'fun'(arg = 1, arg = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | 'fun'(arg = 1, arg = 2)
          | ----------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("list(a = 1, a = 2)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | list(a = 1, a = 2)
          | ------------------ Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("foo(a = 1, a = function(x) 1)"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | foo(a = 1, a = function(x) 1)
          | ----------------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("foo(a = 1, a = (function(x) x + 1))"),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | foo(a = 1, a = (function(x) x + 1))
          | ----------------------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
        // TODO
        // assert!(expect_lint(
        //     "dt[i = 1, i = 2]",
        //     expected_message,
        //     "duplicated_arguments"
        // ));
    }

    #[test]
    fn test_duplicated_arguments_accepted_functions() {
        expect_no_lint(
            "dplyr::mutate(x, a = 1, a = 2)",
            "duplicated_arguments",
            None,
        );
        expect_no_lint("transmute(x, a = 1, a = 2)", "duplicated_arguments", None);
    }

    #[test]
    fn test_duplicated_arguments_no_nested_functions() {
        expect_no_lint(
            "foo(x = {
            bar(a = 1)
            baz(a = 1)
        })",
            "duplicated_arguments",
            None,
        );
    }

    #[test]
    fn test_duplicated_arguments_no_args() {
        expect_no_lint("foo()", "duplicated_arguments", None);
    }

    // ---- Rule-specific config tests ----

    #[test]
    fn test_skipped_functions_replaces_defaults() {
        // With custom skipped-functions = ["list"], only "list" is skipped.
        // Default-skipped "c" should now lint.
        let settings = settings_with_options(DuplicatedArgumentsOptions {
            skipped_functions: Some(vec!["list".to_string()]),
            extend_skipped_functions: None,
        });

        // "list" is in the custom list -> no lint
        expect_no_lint_with_settings(
            "list(a = 1, a = 2)",
            "duplicated_arguments",
            None,
            settings.clone(),
        );

        // "c" is NOT in the custom list -> now lints (was default-skipped)
        assert_snapshot!(
            snapshot_lint_with_settings("c(a = 1, a = 2)", settings),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | c(a = 1, a = 2)
          | --------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_extend_skipped_functions_adds_to_defaults() {
        // extend-skipped-functions = ["my_fun"] -> defaults + "my_fun"
        let settings = settings_with_options(DuplicatedArgumentsOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["my_fun".to_string()]),
        });

        // "my_fun" is in the extended list -> no lint
        expect_no_lint_with_settings(
            "my_fun(a = 1, a = 2)",
            "duplicated_arguments",
            None,
            settings.clone(),
        );

        // Default "c" is still skipped
        expect_no_lint_with_settings(
            "c(a = 1, a = 2)",
            "duplicated_arguments",
            None,
            settings.clone(),
        );

        // "foo" is not in either list -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("foo(a = 1, a = 2)", settings),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | foo(a = 1, a = 2)
          | ----------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_skipped_functions_with_namespaced_call() {
        // duplicated_arguments extracts just the RHS of pkg::fun, so
        // skipping "my_fun" also skips "pkg::my_fun(...)".
        let settings = settings_with_options(DuplicatedArgumentsOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["my_fun".to_string()]),
        });

        expect_no_lint_with_settings(
            "pkg::my_fun(a = 1, a = 2)",
            "duplicated_arguments",
            None,
            settings,
        );
    }

    #[test]
    fn test_duplicated_arguments_with_interceding_comments() {
        assert_snapshot!(
            snapshot_lint(
            "fun(
                arg # xxx
                = 1,
                arg # yyy
                = 2
              )"), @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | / fun(
        2 | |                 arg # xxx
        ... |
        5 | |                 = 2
        6 | |               )
          | |_______________- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint(
            "fun(
                arg = # xxx
                1,
                arg = # yyy
                2
              )"), @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | / fun(
        2 | |                 arg = # xxx
        ... |
        5 | |                 2
        6 | |               )
          | |_______________- Avoid duplicate arguments in function calls. Duplicated argument(s): "arg".
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_namespaced_value_in_config_does_not_match_plain_call() {
        // If the user puts "mypkg::myfun" in the config, only the function name
        // is matched (i.e. "myfun"), so a plain call to `myfun(...)` should NOT
        // match "mypkg::myfun".
        let settings = settings_with_options(DuplicatedArgumentsOptions {
            skipped_functions: Some(vec!["mypkg::myfun".to_string()]),
            extend_skipped_functions: None,
        });

        let code = r#"myfun(a = 1, a = 1)"#;
        assert_snapshot!(
            snapshot_lint_with_settings(code, settings),
            @r#"
        warning: duplicated_arguments
         --> <test>:1:1
          |
        1 | myfun(a = 1, a = 1)
          | ------------------- Avoid duplicate arguments in function calls. Duplicated argument(s): "a".
          |
        Found 1 error.
        "#
        );
    }
}
