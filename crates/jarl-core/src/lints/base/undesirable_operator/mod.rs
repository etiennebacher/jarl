pub(crate) mod options;
pub(crate) mod undesirable_operator;

#[cfg(test)]
mod tests {
    use crate::lints::base::undesirable_operator::options::ResolvedUndesirableOperatorOptions;
    use crate::lints::base::undesirable_operator::options::UndesirableOperatorOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "undesirable_operator", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "undesirable_operator", None, Some(settings))
    }

    fn settings_with_options(options: UndesirableOperatorOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    undesirable_operator: ResolvedUndesirableOperatorOptions::resolve(Some(
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
    fn test_no_lint_undesirable_operator() {
        expect_no_lint("x + y", "undesirable_operator", None);
        expect_no_lint("x |> f()", "undesirable_operator", None);
        expect_no_lint("x %>% f()", "undesirable_operator", None);
        expect_no_lint("utils::f()", "undesirable_operator", None);
        expect_no_lint("`+`(x, y)", "undesirable_operator", None);
        expect_no_lint("# x <<- 1", "undesirable_operator", None);
        expect_no_lint("x <- 'x <<- 1'", "undesirable_operator", None);
    }

    #[test]
    fn test_lint_undesirable_operator() {
        assert_snapshot!(
            snapshot_lint("x <<- 1"),
            @"
        warning: undesirable_operator
         --> <test>:1:3
          |
        1 | x <<- 1
          |   --- `<<-` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1 ->> x"),
            @"
        warning: undesirable_operator
         --> <test>:1:3
          |
        1 | 1 ->> x
          |   --- `->>` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("pkg:::fun()"),
            @"
        warning: undesirable_operator
         --> <test>:1:4
          |
        1 | pkg:::fun()
          |    --- `:::` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("`:::`(pkg, fun)"),
            @"
        warning: undesirable_operator
         --> <test>:1:1
          |
        1 | `:::`(pkg, fun)
          | ----- `:::` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_custom_operators_replace_defaults() {
        let settings = settings_with_options(UndesirableOperatorOptions {
            operators: Some(vec!["$".to_string()]),
            extend_operators: None,
        });

        expect_no_lint_with_settings("x <<- 1", "undesirable_operator", None, settings.clone());

        assert_snapshot!(
            snapshot_lint_with_settings("x$y", settings),
            @"
        warning: undesirable_operator
         --> <test>:1:2
          |
        1 | x$y
          |  - `$` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_extend_operators() {
        let settings = settings_with_options(UndesirableOperatorOptions {
            operators: None,
            extend_operators: Some(vec!["$".to_string()]),
        });

        assert_snapshot!(
            snapshot_lint_with_settings("x <<- 1", settings.clone()),
            @"
        warning: undesirable_operator
         --> <test>:1:3
          |
        1 | x <<- 1
          |   --- `<<-` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint_with_settings("x$y", settings),
            @"
        warning: undesirable_operator
         --> <test>:1:2
          |
        1 | x$y
          |  - `$` is listed as an undesirable operator.
          |
        Found 1 error.
        "
        );
    }
}
