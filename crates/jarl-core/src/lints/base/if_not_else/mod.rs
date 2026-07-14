pub(crate) mod if_not_else;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use crate::lints::base::if_not_else::options::IfNotElseOptions;
    use crate::lints::base::if_not_else::options::ResolvedIfNotElseOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "if_not_else", None)
    }

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
        assert_snapshot!(snapshot_lint("if (!A) x else y"), @"
        warning: if_not_else
         --> <test>:1:1
          |
        1 | if (!A) x else y
          | ---------------- Negating the condition like `if (!A) y else x` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if (A) x else y`
        Found 1 error.
        ");
        // The outer `if` is an else-if (skipped); the inner `if (!B) y else z` lints.
        assert_snapshot!(snapshot_lint("if (!A) x else if (!B) y else z"), @"
        warning: if_not_else
         --> <test>:1:16
          |
        1 | if (!A) x else if (!B) y else z
          |                ---------------- Negating the condition like `if (!A) y else x` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if (A) x else y`
        Found 1 error.
        ");
        // `!` is still the outer operator of a more complex expression.
        assert_snapshot!(snapshot_lint("if (!x %in% 1:10) y else z"), @"
        warning: if_not_else
         --> <test>:1:1
          |
        1 | if (!x %in% 1:10) y else z
          | -------------------------- Negating the condition like `if (!A) y else x` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if (A) x else y`
        Found 1 error.
        ");
    }

    #[test]
    fn test_skips_ifelse_and_friends() {
        for fun in ["ifelse", "fifelse", "if_else"] {
            expect_no_lint(&format!("{fun}(!A | B, x, y)"), "if_not_else", None);
            expect_no_lint(&format!("{fun}(A, !x, y)"), "if_not_else", None);
            // Double negation, particularly relevant for `if_else()`.
            expect_no_lint(&format!("{fun}(!!A, x, y)"), "if_not_else", None);
        }
    }

    #[test]
    fn test_blocks_ifelse_and_friends() {
        assert_snapshot!(snapshot_lint("ifelse(!A, x, y)"), @"
        warning: if_not_else
         --> <test>:1:1
          |
        1 | ifelse(!A, x, y)
          | ---------------- Negating the condition like `ifelse(!A, y, x)` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `ifelse(A, x, y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("fifelse(!A, x, y)"), @"
        warning: if_not_else
         --> <test>:1:1
          |
        1 | fifelse(!A, x, y)
          | ----------------- Negating the condition like `fifelse(!A, y, x)` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `fifelse(A, x, y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if_else(!A, x, y)"), @"
        warning: if_not_else
         --> <test>:1:1
          |
        1 | if_else(!A, x, y)
          | ----------------- Negating the condition like `if_else(!A, y, x)` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if_else(A, x, y)`.
        Found 1 error.
        ");
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
        assert_snapshot!(
            snapshot_lint(
                "{
    if (!A) x else B
    ifelse(!A, x, y)
    fifelse(!A, x, y)
    if_else(!A, x, y)
}"
            ),
            @"
        warning: if_not_else
         --> <test>:2:5
          |
        2 |     if (!A) x else B
          |     ---------------- Negating the condition like `if (!A) y else x` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if (A) x else y`
        warning: if_not_else
         --> <test>:3:5
          |
        3 |     ifelse(!A, x, y)
          |     ---------------- Negating the condition like `ifelse(!A, y, x)` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `ifelse(A, x, y)`.
        warning: if_not_else
         --> <test>:4:5
          |
        4 |     fifelse(!A, x, y)
          |     ----------------- Negating the condition like `fifelse(!A, y, x)` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `fifelse(A, x, y)`.
        warning: if_not_else
         --> <test>:5:5
          |
        5 |     if_else(!A, x, y)
          |     ----------------- Negating the condition like `if_else(!A, y, x)` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if_else(A, x, y)`.
        Found 4 errors.
        "
        );
    }

    #[test]
    fn test_exceptions_argument() {
        // With no exceptions, negated `is.null()` is flagged.
        let settings = settings_with_options(IfNotElseOptions {
            exceptions: Some(vec![]),
            extend_exceptions: None,
        });
        assert_snapshot!(
            format_diagnostics_with_settings(
                "if (!is.null(x)) x else y",
                "if_not_else",
                None,
                Some(settings),
            ),
            @"
        warning: if_not_else
         --> <test>:1:1
          |
        1 | if (!is.null(x)) x else y
          | ------------------------- Negating the condition like `if (!A) y else x` can be hard to read.
          |
          = help: Remove the negation and swap branches, such as `if (A) x else y`
        Found 1 error.
        "
        );

        // With `foo` as the only exception, negated `foo()` is allowed.
        let settings = settings_with_options(IfNotElseOptions {
            exceptions: Some(vec!["foo".to_string()]),
            extend_exceptions: None,
        });
        expect_no_lint_with_settings("if (!foo(x)) y else z", "if_not_else", None, settings);
    }
}
