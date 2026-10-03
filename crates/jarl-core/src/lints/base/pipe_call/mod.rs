pub(crate) mod pipe_call;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

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
        assert_snapshot!(format_diagnostics(
            "x %>% sum %>% mean\nx %T>% print\nx %<>% sort\nx %!>% summary",
            "pipe_call",
            None,
        ));
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
