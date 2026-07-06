pub(crate) mod nested_pipe;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use crate::lints::base::nested_pipe::options::NestedPipeOptions;
    use crate::lints::base::nested_pipe::options::ResolvedNestedPipeOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "nested_pipe", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "nested_pipe", None, Some(settings))
    }

    /// Build a `Settings` with custom `NestedPipeOptions`.
    fn settings_with_options(options: NestedPipeOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    nested_pipe: ResolvedNestedPipeOptions::resolve(Some(&options)).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_no_lint_nested_pipe() {
        // Top-level pipe chains are not nested.
        expect_no_lint("a %>% b() %>% c()", "nested_pipe", None);
        expect_no_lint("a |> b() |> c()", "nested_pipe", None);

        // Assignment right-hand sides are not nested in a call.
        expect_no_lint("out <- a %>% b()", "nested_pipe", None);
        expect_no_lint(
            "foo <- function(x) {\n  out <- a %>% b()\n  return(out)\n}",
            "nested_pipe",
            None,
        );

        // Output positions of `switch()` are allowed.
        expect_no_lint("switch(x, a = x %>% foo())", "nested_pipe", None);
        expect_no_lint("switch(x, a = x, x %>% foo())", "nested_pipe", None);

        // `try`, `tryCatch` and `withCallingHandlers` are skipped by default.
        expect_no_lint("try(x %>% foo())", "nested_pipe", None);
        expect_no_lint("tryCatch(x %>% foo())", "nested_pipe", None);
        expect_no_lint("withCallingHandlers(x %>% foo())", "nested_pipe", None);
    }

    #[test]
    fn test_lint_nested_pipe() {
        assert_snapshot!(
            snapshot_lint("print(a %>% filter(b > c))"),
            @"
        warning: nested_pipe
         --> <test>:1:7
          |
        1 | print(a %>% filter(b > c))
          |       ------------------- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        Found 1 error.
        "
        );

        // Native pipes are handled as well.
        assert_snapshot!(
            snapshot_lint("print(a |> filter(b > c))"),
            @"
        warning: nested_pipe
         --> <test>:1:7
          |
        1 | print(a |> filter(b > c))
          |       ------------------ Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_nested_pipe_multiple() {
        assert_snapshot!(
            snapshot_lint(
                "
bind_rows(
  a %>% select(b),
  c %>%
    select(d),
  e %>%
    select(f) %>%
    filter(g > 0),
  h %>% filter(i < 0)
)"
            ),
            @"
        warning: nested_pipe
         --> <test>:3:3
          |
        3 |   a %>% select(b),
          |   --------------- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        warning: nested_pipe
         --> <test>:4:3
          |
        4 | /   c %>%
        5 | |     select(d),
          | |_____________- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        warning: nested_pipe
         --> <test>:6:3
          |
        6 | /   e %>%
        7 | |     select(f) %>%
        8 | |     filter(g > 0),
          | |_________________- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        warning: nested_pipe
         --> <test>:9:3
          |
        9 |   h %>% filter(i < 0)
          |   ------------------- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        Found 4 errors.
        "
        );
    }

    #[test]
    fn test_lint_nested_pipe_switch_input() {
        // The first argument of `switch()` is an input position and is linted.
        assert_snapshot!(
            snapshot_lint("
switch(
    x %>% foo(),
    a = x
)"),
            @"
        warning: nested_pipe
         --> <test>:3:5
          |
        3 |     x %>% foo(),
          |     ----------- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_nested_pipe_skipped_functions() {
        // Emptying the skip list makes `try()` lint again.
        let settings = settings_with_options(NestedPipeOptions {
            skipped_functions: Some(vec![]),
            extend_skipped_functions: None,
        });
        assert_snapshot!(
            snapshot_lint_with_settings("try(x %>% foo())", settings),
            @"
        warning: nested_pipe
         --> <test>:1:5
          |
        1 | try(x %>% foo())
          |     ----------- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        Found 1 error.
        "
        );

        // Entirely redefine the list of skipped functions.
        let settings = settings_with_options(NestedPipeOptions {
            skipped_functions: Some(vec!["print".to_string()]),
            extend_skipped_functions: None,
        });
        assert_snapshot!(
            snapshot_lint_with_settings("
            try(x %>% foo())
            print(x %>% foo())", settings),
            @"
        warning: nested_pipe
         --> <test>:2:17
          |
        2 |             try(x %>% foo())
          |                 ----------- Don't nest pipes inside other calls.
          |
          = help: Extract the pipe into its own statement and pass the result to the call.
        Found 1 error.
        "
        );

        // Extend the list of skipped functions.
        let settings = settings_with_options(NestedPipeOptions {
            skipped_functions: None,
            extend_skipped_functions: Some(vec!["print".to_string()]),
        });
        assert_snapshot!(
            snapshot_lint_with_settings("
            try(x %>% foo())
            print(x %>% foo())", settings),
            @"All checks passed!"
        );
    }
}
