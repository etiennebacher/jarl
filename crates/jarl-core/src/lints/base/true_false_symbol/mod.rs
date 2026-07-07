pub(crate) mod options;
pub(crate) mod true_false_symbol;

#[cfg(test)]
mod tests {
    use crate::lints::base::true_false_symbol::options::ResolvedTrueFalseSymbolOptions;
    use crate::lints::base::true_false_symbol::options::TrueFalseSymbolOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;

    /// Build a `Settings` with custom `TrueFalseSymbolOptions`.
    fn settings_with_options(options: TrueFalseSymbolOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    true_false_symbol: ResolvedTrueFalseSymbolOptions::resolve(Some(&options))
                        .unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    // TODO: I guess this should only be linted if --unsafe-fixes is passed?
    // #[test]
    // fn test_lint_true_false_symbol() {
    //     let expected_message = "can be confused with variable names";
    //     expect_lint("T", expected_message, "true_false_symbol", None);
    //     expect_lint("F", expected_message, "true_false_symbol", None);
    //     expect_lint("T = 42", expected_message, "true_false_symbol", None);
    //     expect_lint("F = 42", expected_message, "true_false_symbol", None);
    //     expect_lint(
    //         "for (i in 1:10) {x <- c(T, TRUE, F, FALSE)}",
    //         expected_message,
    //         "true_false_symbol", None,
    //     );
    //     expect_lint("DF$bool <- T", expected_message, "true_false_symbol", None);
    //     expect_lint("S4@bool <- T", expected_message, "true_false_symbol", None);
    //     expect_lint("sum(x, na.rm = T)", expected_message, "true_false_symbol", None);
    // }

    #[test]
    fn test_no_lint_true_false_symbol() {
        expect_no_lint("TRUE", "true_false_symbol", None);
        expect_no_lint("FALSE", "true_false_symbol", None);
        expect_no_lint("T()", "true_false_symbol", None);
        expect_no_lint("F()", "true_false_symbol", None);
        expect_no_lint("x <- \"T\"", "true_false_symbol", None);
        expect_no_lint("mtcars$F", "true_false_symbol", None);
        expect_no_lint("mtcars$T", "true_false_symbol", None);
    }
    #[test]
    fn test_true_false_symbol_in_formulas() {
        let _expected_message = "can be confused with variable names";
        // TODO
        // assert!(expect_lint(
        //     "lm(weight ~ var + foo(x, arg = T), data)",
        //     expected_message,
        //     "true_false_symbol", None
        // ));

        expect_no_lint("lm(weight ~ T, data)", "true_false_symbol", None);
        expect_no_lint("lm(weight ~ F, data)", "true_false_symbol", None);
        // TODO
        // expect_no_lint("lm(weight ~ T + var", "true_false_symbol", None);
        // expect_no_lint("lm(weight ~ A + T | var", "true_false_symbol", None);
        // expect_no_lint("lm(weight ~ var | A + T", "true_false_symbol", None);
        // TODO
        // expect_no_lint(
        //     "lm(weight ~ var + var2 + T, data)",
        //     "true_false_symbol", None
        // );
        expect_no_lint("lm(T ~ weight, data)", "true_false_symbol", None);
    }

    #[test]
    fn test_true_false_symbol_skipped_functions() {
        let settings = settings_with_options(TrueFalseSymbolOptions {
            skipped_functions: Some(vec!["foo".to_string()]),
        });
        expect_no_lint_with_settings("foo(T)", "true_false_symbol", None, settings.clone());
        expect_no_lint_with_settings("foo(x, y = F)", "true_false_symbol", None, settings.clone());
        // Nested inside a skipped call is also allowed.
        expect_no_lint_with_settings("foo(bar(T))", "true_false_symbol", None, settings);

        // Calls that are not skipped are still linted.
        let settings = settings_with_options(TrueFalseSymbolOptions {
            skipped_functions: Some(vec!["foo".to_string()]),
        });
        assert!(
            format_diagnostics_with_settings("bar(T)", "true_false_symbol", None, Some(settings))
                .contains("true_false_symbol")
        );
    }

    // TODO
    // #[test]
    // fn test_true_false_symbol_in_function_args() {
    //     expect_no_lint("myfun <- function(T) {}", "true_false_symbol", None));
    //     expect_no_lint("myfun <- function(F) {}", "true_false_symbol", None));
    // }

    // #[test]
    // fn test_true_false_symbol_in_named_vectors() {
    //     expect_no_lint("c(T = 'foo', F = 'foo')", "true_false_symbol", None));
    // }
}
