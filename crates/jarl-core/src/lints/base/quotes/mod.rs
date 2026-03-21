pub(crate) mod quotes;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::quotes::QuotesOptions;
    use crate::rule_options::quotes::ResolvedQuotesOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "quotes", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "quotes", None, Some(settings))
    }

    /// Build a `Settings` with custom `QuotesOptions`.
    fn settings_with_options(options: QuotesOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    quotes: ResolvedQuotesOptions::resolve(Some(&options)).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_quotes_double_quote_allows_needed_and_preferred_forms() {
        expect_no_lint("foo", "quotes", None);
        expect_no_lint("\"bar\"", "quotes", None);
        expect_no_lint("\"'blah'\"", "quotes", None);
        expect_no_lint("'\"'", "quotes", None);
        expect_no_lint("'\"hello\"'", "quotes", None);
        expect_no_lint("R\"(say 'hello world')\"", "quotes", None);
    }

    #[test]
    fn test_quotes_standard_double_quote_lints() {
        assert_snapshot!(
            snapshot_lint("'hi'"),
            @"
        warning: quotes
         --> <test>:1:1
          |
        1 | 'hi'
          | ^^^^ Prefer double quotes for string delimiters.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("fun('hello')"),
            @"
        warning: quotes
         --> <test>:1:5
          |
        1 | fun('hello')
          |     ^^^^^^^ Prefer double quotes for string delimiters.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x <- 'test'"),
            @"
        warning: quotes
         --> <test>:1:6
          |
        1 | x <- 'test'
          |      ^^^^^^ Prefer double quotes for string delimiters.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint( "c(\n  'abc',\n  \"def\",\n  'ghi'\n)"),
            @"
        warning: quotes
         --> <test>:2:3
          |
        2 |   'abc',
          |   ^^^^^ Prefer double quotes for string delimiters.
        warning: quotes
         --> <test>:4:3
          |
        4 |   'ghi'
          |   ^^^^^ Prefer double quotes for string delimiters.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_quotes_single_quote_allows_needed_and_preferred_forms() {
        let settings = settings_with_options(QuotesOptions { quote: Some("single".to_string()) });

        expect_no_lint_with_settings("foo", "quotes", None, settings.clone());
        expect_no_lint_with_settings("'blah'", "quotes", None, settings.clone());
        expect_no_lint_with_settings("'\"bar\"'", "quotes", None, settings.clone());
        expect_no_lint_with_settings("\"'blah\"", "quotes", None, settings.clone());
        expect_no_lint_with_settings("'\"'", "quotes", None, settings.clone());
        expect_no_lint_with_settings("\"'blah'\"", "quotes", None, settings);
    }

    #[test]
    fn test_quotes_standard_single_quote_lints() {
        let settings = settings_with_options(QuotesOptions { quote: Some("single".to_string()) });

        assert_snapshot!(
            snapshot_lint_with_settings("\"blah\"", settings.clone()),
            @r#"
        warning: quotes
         --> <test>:1:1
          |
        1 | "blah"
          | ^^^^^^ Prefer single-quotes for string delimiters.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint_with_settings("x <- \"test\"", settings.clone()),
            @r#"
        warning: quotes
         --> <test>:1:6
          |
        1 | x <- "test"
          |      ^^^^^^ Prefer single-quotes for string delimiters.
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_quotes_raw_double_quote_lints() {
        assert_snapshot!(
            snapshot_lint("R'( whoops )'"),
            @"
        warning: quotes
         --> <test>:1:1
          |
        1 | R'( whoops )'
          | ^^^^^^^^^^^^^ Prefer double quotes for string delimiters.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("R'---[ hello ]---'"),
            @"
        warning: quotes
         --> <test>:1:1
          |
        1 | R'---[ hello ]---'
          | ^^^^^^^^^^^^^^^^^^ Prefer double quotes for string delimiters.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("r'{'rawstring'}'"),
            @"
        warning: quotes
         --> <test>:1:1
          |
        1 | r'{'rawstring'}'
          | ^^^^^^^^^^^^^^^^ Prefer double quotes for string delimiters.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_quotes_skip_raw_strings_with_syntax_error() {
        expect_no_lint("R'---[ daisy ]--'", "quotes", None);
        expect_no_lint("r'(hello]'", "quotes", None);
    }

    #[test]
    fn test_quotes_does_not_treat_non_adjacent_r_as_raw_prefix() {
        expect_no_lint("r \"hi\"", "quotes", None);
        expect_no_lint("r; \"hi\"", "quotes", None);
        expect_no_lint("r\n\"hi\"", "quotes", None);
    }

    #[test]
    fn test_quotes_skips_raw_string_includes_preferred() {
        expect_no_lint("r'(rawstring\")'", "quotes", None);
        expect_no_lint("r'(say \"hello world\")'", "quotes", None);
        expect_no_lint("r'(abc)\"def)'", "quotes", None);
    }

    #[test]
    fn test_quotes_fix_output_double_quote() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "'abc'",
                    "fun('abc')",
                    "R'(raw text)'",
                    "R'---[ raw text ]---'",
                ],
                "quotes",
                None
            )
        );
    }

    #[test]
    fn test_quotes_fix_output_single_quote() {
        let settings = settings_with_options(QuotesOptions { quote: Some("single".to_string()) });

        assert_snapshot!(
            "fix_output_single_quote",
            get_fixed_text_with_settings(
                vec![
                    "\"abc\"",
                    "fun(\"abc\")",
                    "R\"(raw text)\"",
                    "R\"---[ raw text ]---\"",
                ],
                "quotes",
                None,
                Some(settings)
            )
        );
    }

    #[test]
    fn test_quotes_no_fix_double_quote() {
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(
                vec![
                    "r'(abc)\"def)'",
                    "r'(\"rawstring\")'",
                    "r'-(\"hello\")-'",
                    "'\"abc\"'",
                ],
                "quotes",
                None
            )
        );
    }

    #[test]
    fn test_quotes_no_fix_single_quote() {
        let settings = settings_with_options(QuotesOptions { quote: Some("single".to_string()) });

        assert_snapshot!(
            "no_fix_output_single_quote",
            get_fixed_text_with_settings(
                vec![
                    "r\"('rawstring')\"",
                    "r\"-( 'hello' )-\"",
                    "r\"(abc)'def)\"",
                    "\"'abc'\"",
                ],
                "quotes",
                None,
                Some(settings)
            )
        );
    }
}
