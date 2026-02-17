pub(crate) mod undesirable_function;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::undesirable_function::ResolvedUndesirableFunctionOptions;
    use crate::rule_options::undesirable_function::UndesirableFunctionOptions;
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
            @r"
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
            @r"
        warning: undesirable_function
         --> <test>:1:1
          |
        1 | utils::browser()
          | ---------------- `browser()` is listed as an undesirable function.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_custom_functions() {
        let settings = settings_with_options(UndesirableFunctionOptions {
            functions: Some(vec!["debug".to_string()]),
            extend_functions: None,
        });

        // "browser" is no longer in the list -> no lint
        expect_no_lint_with_settings("browser()", "undesirable_function", None, settings.clone());

        // "debug" is in the custom list -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("debug(x)", settings),
            @r"
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
            extend_functions: Some(vec!["debug".to_string()]),
        });

        // "browser" is still in the defaults -> lints
        assert_snapshot!(
            snapshot_lint_with_settings("browser()", settings.clone()),
            @r"
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
            @r"
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
}
