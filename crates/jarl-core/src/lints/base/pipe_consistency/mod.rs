pub(crate) mod options;
pub(crate) mod pipe_consistency;

#[cfg(test)]
mod tests {
    use crate::lints::base::pipe_consistency::options::{
        PreferredPipe, ResolvedPipeConsistencyOptions,
    };
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "pipe_consistency", Some("4.2"))
    }

    fn settings_with_preferred(pipe: PreferredPipe) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    pipe_consistency: ResolvedPipeConsistencyOptions { pipe },
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_lint_pipe_consistency_default_base() {
        assert_snapshot!(
            snapshot_lint("x %>% f()"),
            @r"
        warning: pipe_consistency
         --> <test>:1:3
          |
        1 | x %>% f()
          |   --- `%>%` is inconsistent with the preferred pipe `|>`.
          |
          = help: Use `|>` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("data %>%\n  transform(a = x / 2) |>\n  plot()"),
            @r"
        warning: pipe_consistency
         --> <test>:1:6
          |
        1 | data %>%
          |      --- `%>%` is inconsistent with the preferred pipe `|>`.
          |
          = help: Use `|>` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "x %>% f()",
                    "x %>% f(y = .)",
                    "data %>%\n  transform(a = x / 2) |>\n  plot()",
                ],
                "pipe_consistency",
                Some("4.2"),
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_pipe_consistency_default_base() {
        // base pipe alone is fine
        expect_no_lint("x |> f()", "pipe_consistency", Some("4.2"));
        // no pipe at all is fine
        expect_no_lint("x + y", "pipe_consistency", Some("4.2"));
        // assignment is not a pipe
        expect_no_lint("x <- y", "pipe_consistency", Some("4.2"));

        // `.` as unnamed argument has no `_` equivalent
        expect_no_lint("x %>% f(.)", "pipe_consistency", Some("4.2"));
        // `.` appearing more than once can't be expressed with `_`
        expect_no_lint("x %>% f(a = ., b = .)", "pipe_consistency", Some("4.2"));
        // `.` nested below the top-level call argument is not supported
        expect_no_lint("x %>% f(g(.))", "pipe_consistency", Some("4.2"));

        // Disabled below R 4.2 (the `_` placeholder requires 4.2)
        expect_no_lint("x %>% f()", "pipe_consistency", Some("4.1"));
    }

    #[test]
    fn test_lint_pipe_consistency_prefer_magrittr() {
        let settings = settings_with_preferred(PreferredPipe::Magrittr);

        assert_snapshot!(
            format_diagnostics_with_settings(
                "x |> f()",
                "pipe_consistency",
                Some("4.2"),
                Some(settings.clone()),
            ),
            @r"
        warning: pipe_consistency
         --> <test>:1:3
          |
        1 | x |> f()
          |   -- `|>` is inconsistent with the preferred pipe `%>%`.
          |
          = help: Use `%>%` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output_prefer_magrittr",
            get_unsafe_fixed_text_with_settings(
                vec!["x |> f()", "x |> f(y = _)"],
                "pipe_consistency",
                Some("4.2"),
                Some(settings),
            )
        );
    }

    #[test]
    fn test_no_lint_pipe_consistency_prefer_magrittr() {
        let settings = settings_with_preferred(PreferredPipe::Magrittr);

        // `%>%` is the pipe form
        expect_no_lint_with_settings("x %>% f()", "pipe_consistency", Some("4.2"), settings);
    }

    #[test]
    fn test_pipe_consistency_with_comments_no_fix() {
        // Detect the lint but skip the fix when comments are present.
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "x %>% # trailing comment\n  f()",
                    "x %>%\n  # leading on rhs\n  f()",
                ],
                "pipe_consistency",
                Some("4.2"),
                None,
            )
        );
    }
}
