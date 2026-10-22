pub(crate) mod pipe_call;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "pipe_call", None)
    }

    #[test]
    fn test_no_lint_pipe_call() {
        for code in [
            "sum(x)",
            "x + sum",
            "x %other% sum",
            "x %$% sum",
            "x |> sum",
            "x |> sum()",
            "x %>% sum()",
            "x %!>% sum()",
            "x %T>% print()",
            "x %<>% sort()",
            "x %>% (sum)",
            "x %>% base::sum",
            "x %>% base:::sum",
            "x %>% .$column",
            "x %>% functions$sum",
            "x %>% functions[[1]]",
            "x %>% functions@sum",
            "x %>% { sum(.) }",
            "x %>% function(y) sum(y)",
            "x %>% TRUE",
            "x %>% 1",
            "x %>% 'sum'",
        ] {
            expect_no_lint(code, "pipe_call", None);
        }
    }

    #[test]
    fn test_lint_pipe_call() {
        assert_snapshot!(
            snapshot_lint("x %>% sum %>% mean"),
            @"
        warning: pipe_call
         --> <test>:1:15
          |
        1 | x %>% sum %>% mean
          |               ---- Use an explicit call on the right-hand side of a magrittr pipe.
          |
          = help: Use `mean()` instead.
        warning: pipe_call
         --> <test>:1:7
          |
        1 | x %>% sum %>% mean
          |       --- Use an explicit call on the right-hand side of a magrittr pipe.
          |
          = help: Use `sum()` instead.
        Found 2 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %T>% print"),
            @"
        warning: pipe_call
         --> <test>:1:8
          |
        1 | x %T>% print
          |        ----- Use an explicit call on the right-hand side of a magrittr pipe.
          |
          = help: Use `print()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %<>% sort"),
            @"
        warning: pipe_call
         --> <test>:1:8
          |
        1 | x %<>% sort
          |        ---- Use an explicit call on the right-hand side of a magrittr pipe.
          |
          = help: Use `sort()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %!>% summary"),
            @"
        warning: pipe_call
         --> <test>:1:8
          |
        1 | x %!>% summary
          |        ------- Use an explicit call on the right-hand side of a magrittr pipe.
          |
          = help: Use `summary()` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_pipe_call() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x %>% sum %>% mean() %>% print",
                    "x %T>% print",
                    "x %<>% sort",
                    "x %!>% summary",
                    "x %>% `+`",
                    "x %>% `some function`",
                    "x %>% caf\u{e9}",
                    "x %>% .",
                    "# leading\nx %>% # before function\n  sum # trailing",
                    "f(x %>% sum, y %>% mean)",
                ],
                "pipe_call",
                None,
            )
        );
    }
}
