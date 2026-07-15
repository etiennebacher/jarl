pub(crate) mod options;
pub(crate) mod positional_arguments;

#[cfg(test)]
mod tests {
    use crate::lints::base::positional_arguments::options::PositionalArgumentsOptions;
    use crate::lints::base::positional_arguments::options::ResolvedPositionalArgumentsOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "positional_arguments", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "positional_arguments", None, Some(settings))
    }

    /// Build a `Settings` with custom `PositionalArgumentsOptions`.
    fn settings_with_options(options: PositionalArgumentsOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    positional_arguments: ResolvedPositionalArgumentsOptions::resolve(Some(
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
    fn test_no_lint_positional_arguments() {
        // At most three positional arguments are allowed by default.
        expect_no_lint("foo()", "positional_arguments", None);
        expect_no_lint("foo(1)", "positional_arguments", None);
        expect_no_lint("foo(x)", "positional_arguments", None);
        expect_no_lint("foo(1, 2, 3)", "positional_arguments", None);

        // Naming the extra arguments makes the call compliant.
        expect_no_lint("foo(1, 2, 3, w = 4)", "positional_arguments", None);
        expect_no_lint(
            "foo(w = 1, x = 2, y = 3, z = 4)",
            "positional_arguments",
            None,
        );

        // Variadic functions are skipped by default.
        expect_no_lint("c(1, 2, 3, 4)", "positional_arguments", None);
        expect_no_lint("paste(a, b, c, d)", "positional_arguments", None);
        expect_no_lint("paste0(a, b, c, d)", "positional_arguments", None);
    }

    #[test]
    fn test_lint_positional_arguments() {
        assert_snapshot!(
            snapshot_lint("grepl(\"a\", x, TRUE, FALSE)"),
            @r#"
        warning: positional_arguments
         --> <test>:1:1
          |
        1 | grepl("a", x, TRUE, FALSE)
          | -------------------------- Calling a function with 4 positional arguments can be hard to read and is prone to mistakes.
          |
          = help: Name the arguments to clarify what each value refers to.
        Found 1 error.
        "#
        );

        // Named arguments are not counted, only the positional ones.
        assert_snapshot!(
            snapshot_lint("foo(1, 2, 3, 4, w = 5)"),
            @"
        warning: positional_arguments
         --> <test>:1:1
          |
        1 | foo(1, 2, 3, 4, w = 5)
          | ---------------------- Calling a function with 4 positional arguments can be hard to read and is prone to mistakes.
          |
          = help: Name the arguments to clarify what each value refers to.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_positional_arguments_max_positional_args() {
        // Raising the threshold allows more positional arguments.
        let settings = settings_with_options(PositionalArgumentsOptions {
            max_positional_args: Some(4),
            ..Default::default()
        });
        assert_snapshot!(
            snapshot_lint_with_settings("foo(1, 2, 3, 4)", settings),
            @"All checks passed!"
        );

        // Lowering the threshold reports calls with a single positional argument.
        let settings = settings_with_options(PositionalArgumentsOptions {
            max_positional_args: Some(0),
            ..Default::default()
        });
        assert_snapshot!(
            snapshot_lint_with_settings("foo(1)", settings),
            @"
        warning: positional_arguments
         --> <test>:1:1
          |
        1 | foo(1)
          | ------ Calling a function with 1 positional argument can be hard to read and is prone to mistakes.
          |
          = help: Name the arguments to clarify what each value refers to.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_positional_arguments_skipped_functions() {
        // Emptying the skip list makes `c()` lint again.
        let settings = settings_with_options(PositionalArgumentsOptions {
            skipped_functions: Some(vec![]),
            ..Default::default()
        });
        assert_snapshot!(
            snapshot_lint_with_settings("c(1, 2, 3, 4)", settings),
            @"
        warning: positional_arguments
         --> <test>:1:1
          |
        1 | c(1, 2, 3, 4)
          | ------------- Calling a function with 4 positional arguments can be hard to read and is prone to mistakes.
          |
          = help: Name the arguments to clarify what each value refers to.
        Found 1 error.
        "
        );

        // Entirely redefine the list of skipped functions.
        let settings = settings_with_options(PositionalArgumentsOptions {
            skipped_functions: Some(vec!["foo".to_string()]),
            ..Default::default()
        });
        assert_snapshot!(
            snapshot_lint_with_settings("foo(1, 2, 3, 4)\nc(1, 2, 3, 4)", settings),
            @"
        warning: positional_arguments
         --> <test>:2:1
          |
        2 | c(1, 2, 3, 4)
          | ------------- Calling a function with 4 positional arguments can be hard to read and is prone to mistakes.
          |
          = help: Name the arguments to clarify what each value refers to.
        Found 1 error.
        "
        );

        // Extend the list of skipped functions.
        let settings = settings_with_options(PositionalArgumentsOptions {
            extend_skipped_functions: Some(vec!["foo".to_string()]),
            ..Default::default()
        });
        expect_no_lint_with_settings("foo(1, 2, 3, 4)", "positional_arguments", None, settings);
    }
}
