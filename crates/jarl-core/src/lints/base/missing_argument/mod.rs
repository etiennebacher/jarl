pub(crate) mod missing_argument;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::missing_argument::MissingArgumentOptions;
    use crate::rule_options::missing_argument::ResolvedMissingArgumentOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "missing_argument", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "missing_argument", None, Some(settings))
    }

    /// Build a `Settings` with custom `MissingArgumentOptions`.
    fn settings_with_options(options: MissingArgumentOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    missing_argument: ResolvedMissingArgumentOptions::resolve(Some(&options))
                        .unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_no_lint_missing_argument() {
        expect_no_lint("paste('a', 'b')", "missing_argument", None);
        expect_no_lint("mean(x)", "missing_argument", None);
        expect_no_lint("f(x = 1, y = 2)", "missing_argument", None);
        expect_no_lint("f(x = 1, 2)", "missing_argument", None);
        expect_no_lint("f()", "missing_argument", None);
        expect_no_lint(
            "switch(type, a = , b = 'ab', c = 'c')",
            "missing_argument",
            None,
        );
    }

    #[test]
    fn test_lint_single_empty_argument() {
        assert_snapshot!(snapshot_lint("f(x, )"));
        assert_snapshot!(snapshot_lint("f('a', , 'b', )"));
        assert_snapshot!(snapshot_lint("f(, 'a', , 'b', )"));
    }

    #[test]
    fn test_skipped_functions_replaces_defaults() {
        let settings = settings_with_options(MissingArgumentOptions {
            skipped_functions: Some(vec!["quote".to_string()]),
            extend_skipped_functions: None,
        });

        // "mutate" is ignored by default but here we overwrote the list
        snapshot_lint_with_settings(
            "
        quote(x, )
        mutate(x, )",
            settings.clone(),
        );
    }

    #[test]
    fn test_extend_skipped_functions_adds_to_defaults() {
        let settings = settings_with_options(MissingArgumentOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["quote".to_string()]),
        });
        snapshot_lint_with_settings(
            "
        quote(x, )
        mutate(x, )",
            settings.clone(),
        );
    }

    #[test]
    fn test_skipped_functions_with_namespaced_call() {
        let settings = settings_with_options(MissingArgumentOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["my_fun".to_string()]),
        });
        expect_no_lint_with_settings("pkg::my_fun(x, )", "missing_argument", None, settings);
    }
}
