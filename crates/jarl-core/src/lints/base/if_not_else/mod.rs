pub(crate) mod if_not_else;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use crate::lints::base::if_not_else::options::IfNotElseOptions;
    use crate::lints::base::if_not_else::options::ResolvedIfNotElseOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;

    /// Build a `Settings` with custom `IfNotElseOptions`.
    fn settings_with_options(options: IfNotElseOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    if_not_else: ResolvedIfNotElseOptions::resolve(Some(&options)).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Assert that the code produces a diagnostic whose message contains `msg`.
    fn expect_message(code: &str, msg: &str) {
        let output = format_diagnostics(code, "if_not_else", None);
        assert!(
            output.contains(msg),
            "Expected diagnostic containing {msg:?} for code {code:?}, got:\n{output}"
        );
    }

    #[test]
    fn test_skips_allowed_usages() {
        // Simple if/else statement is fine.
        expect_no_lint("if (A) x else y", "if_not_else", None);
        // Not a plain negation.
        expect_no_lint("if (!A || B) x else y", "if_not_else", None);
        // No else clause.
        expect_no_lint("if (!A) x", "if_not_else", None);

        // `else if` chains are also OK.
        expect_no_lint("if (!A) x else if (B) y", "if_not_else", None);
        expect_no_lint("if (!A) x else if (B) y else z", "if_not_else", None);
        expect_no_lint("if (A) x else if (B) y else if (!C) z", "if_not_else", None);

        // A `!` in the branches (not the condition) is skipped.
        expect_no_lint("if (A) !x else y", "if_not_else", None);
        expect_no_lint("if (A) x else !y", "if_not_else", None);
    }

    #[test]
    fn test_blocks_simple_usages() {
        expect_message("if (!A) x else y", "Prefer `if (A) x else y`");
        // The outer `if` is an else-if (skipped); the inner `if (!B) y else z` lints.
        expect_message(
            "if (!A) x else if (!B) y else z",
            "Prefer `if (A) x else y`",
        );
        // `!` is still the outer operator of a more complex expression.
        expect_message("if (!x %in% 1:10) y else z", "Prefer `if (A) x else y`");
    }

    #[test]
    fn test_blocks_ifelse_and_friends() {
        for fun in ["ifelse", "fifelse", "if_else"] {
            expect_no_lint(&format!("{fun}(!A | B, x, y)"), "if_not_else", None);
            expect_no_lint(&format!("{fun}(A, !x, y)"), "if_not_else", None);
            expect_message(
                &format!("{fun}(!A, x, y)"),
                &format!("Prefer `{fun}(A, x, y)` to the less-readable"),
            );
            // Double negation, particularly relevant for `if_else()`.
            expect_no_lint(&format!("{fun}(!!A, x, y)"), "if_not_else", None);
        }
    }

    #[test]
    fn test_skips_negated_is_null_and_similar() {
        expect_no_lint("if (!is.null(x)) x else y", "if_not_else", None);
        expect_no_lint("if (!is.na(x)) x else y", "if_not_else", None);
        expect_no_lint("if (!missing(x)) x else y", "if_not_else", None);
        expect_no_lint("ifelse(!is.na(x), x, y)", "if_not_else", None);
    }

    #[test]
    fn test_multiple_lints() {
        let code = "{
    if (!A) x else B
    ifelse(!A, x, y)
    fifelse(!A, x, y)
    if_else(!A, x, y)
}";
        let output = format_diagnostics(code, "if_not_else", None);
        assert!(output.contains("Prefer `if (A) x else y`"));
        assert!(output.contains("Prefer `ifelse"));
        assert!(output.contains("Prefer `fifelse"));
        assert!(output.contains("Prefer `if_else"));
    }

    #[test]
    fn test_exceptions_argument() {
        // With no exceptions, negated `is.null()` is flagged.
        let settings = settings_with_options(IfNotElseOptions {
            exceptions: Some(vec![]),
            extend_exceptions: None,
        });
        let output = format_diagnostics_with_settings(
            "if (!is.null(x)) x else y",
            "if_not_else",
            None,
            Some(settings),
        );
        assert!(output.contains("Prefer `if (A) x else y`"));

        // With `foo` as the only exception, negated `foo()` is allowed.
        let settings = settings_with_options(IfNotElseOptions {
            exceptions: Some(vec!["foo".to_string()]),
            extend_exceptions: None,
        });
        expect_no_lint_with_settings("if (!foo(x)) y else z", "if_not_else", None, settings);
    }
}
