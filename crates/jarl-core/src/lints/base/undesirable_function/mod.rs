pub(crate) mod options;
pub(crate) mod undesirable_function;

#[cfg(test)]
mod tests {
    use crate::lints::base::undesirable_function::options::ResolvedUndesirableFunctionOptions;
    use crate::lints::base::undesirable_function::options::UndesirableFunctionEntry;
    use crate::lints::base::undesirable_function::options::UndesirableFunctionOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "undesirable_function", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "undesirable_function", None, Some(settings))
    }

    fn settings_with_options(options: UndesirableFunctionOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    undesirable_function: ResolvedUndesirableFunctionOptions::resolve(Some(
                        &options,
                    ))
                    .unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_no_lint_undesirable_function() {
        expect_no_lint("print('hello')", "undesirable_function", None);
        expect_no_lint(
            "function(browser = 'firefox')",
            "undesirable_function",
            None,
        );
        expect_no_lint("function(tool = browser)", "undesirable_function", None);
        expect_no_lint("# browser()", "undesirable_function", None);
    }

    #[test]
    fn test_lint_undesirable_function() {
        assert_snapshot!(
            snapshot_lint("browser()"),
            @"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | browser()
          | --------- `browser()` is listed as an undesirable function.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("utils::browser()"),
            @"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | utils::browser()
          | ---------------- `utils::browser()` is listed as an undesirable function.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_custom_functions() {
        let settings = settings_with_options(UndesirableFunctionOptions {
            functions: Some(vec![UndesirableFunctionEntry::Name("debug".to_string())]),
            extend_functions: None,
        });

        // "browser" is no longer in the list -> no lint
        expect_no_lint_with_settings("browser()", "undesirable_function", None, settings.clone());

        // "debug" is in the custom list -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("debug(x)", settings),
            @"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | debug(x)
          | -------- `debug()` is listed as an undesirable function.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_extend_functions() {
        let settings = settings_with_options(UndesirableFunctionOptions {
            functions: None,
            extend_functions: Some(vec![UndesirableFunctionEntry::Name("debug".to_string())]),
        });

        // "browser" is still in the defaults -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("browser()", settings.clone()),
            @"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | browser()
          | --------- `browser()` is listed as an undesirable function.
          |
        Found 1 error.
        "
        );

        // "debug" was added via extend -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("debug(x)", settings),
            @"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | debug(x)
          | -------- `debug()` is listed as an undesirable function.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_custom_messages() {
        let options: UndesirableFunctionOptions = toml::from_str(
            r#"
            extend-functions = [
                { setwd = 'Use here::here().' },
                "sprintf",
                { transmute = 'Use mutate(.keep = "none").' },
            ]
            "#,
        )
        .unwrap();

        let settings = settings_with_options(options);

        assert_snapshot!(
            snapshot_lint_with_settings("setwd()", settings.clone()),
            @"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | setwd()
          | ------- `setwd()` is listed as an undesirable function.
          |
          = help: Use here::here().
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint_with_settings("transmute()", settings),
            @r#"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | transmute()
          | ----------- `transmute()` is listed as an undesirable function.
          |
          = help: Use mutate(.keep = "none").
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_namespaced_function_matches_exactly() {
        let options: UndesirableFunctionOptions =
            toml::from_str(r#"extend-functions = [{ "base::setwd" = "Use here::here()." }]"#)
                .unwrap();
        let settings = settings_with_options(options);
        expect_no_lint_with_settings("setwd()", "undesirable_function", None, settings.clone());
        assert!(snapshot_lint_with_settings("base::setwd()", settings).contains("base::setwd()"));
    }

    #[test]
    fn test_invalid_custom_function_entries_are_rejected() {
        for (config, message) in [
            (
                "extend-functions = [{ 1 = 'Use here::here().' }]",
                "Function name `1` must be a valid R identifier or use the form `package::name` in `[lint.undesirable_function]`.",
            ),
            (
                "extend-functions = [{ true = 'Use here::here().' }]",
                "Function name `true` must be a valid R identifier or use the form `package::name` in `[lint.undesirable_function]`.",
            ),
            (
                "extend-functions = [{ setwd = 1 }]",
                "Suggestion for `setwd` in `[lint.undesirable_function]` must be a string.",
            ),
            (
                "extend-functions = [{ setwd = true }]",
                "Suggestion for `setwd` in `[lint.undesirable_function]` must be a string.",
            ),
            (
                "extend-functions = [{ \"\" = true }]",
                "Function name cannot be empty in `[lint.undesirable_function]`.",
            ),
            (
                "extend-functions = [{ \"  setwd  \" = true }]",
                "Function name `  setwd  ` cannot have leading or trailing whitespace in `[lint.undesirable_function]`.",
            ),
        ] {
            let options: UndesirableFunctionOptions = toml::from_str(config).unwrap();
            assert_eq!(
                ResolvedUndesirableFunctionOptions::resolve(Some(&options))
                    .unwrap_err()
                    .to_string(),
                message
            );
        }
    }
}
