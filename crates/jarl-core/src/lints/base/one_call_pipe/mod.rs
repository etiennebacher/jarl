pub(crate) mod one_call_pipe;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "one_call_pipe", None)
    }

    #[test]
    fn test_no_lint_one_call_pipe_boundaries() {
        // A call anywhere in the left expression counts, not only a call at
        // its root. Parentheses and infix/subset expressions don't hide it.
        for code in [
            "(f(x) + 1) |> abs()",
            "(x + f(y)) |> abs()",
            "(x[transform(y)]) |> abs()",
            "(pkg::prepare(x) + 1) %>% abs()",
            "rowSums(x) %>% mean()",
            "foo(bar(x)) |> abs()",
            "f(x) |> g()",
            "(f(x)) |> g()",
            "(x + f(y) + g(z)) |> h()",
            "(x + 1) |> f(g())",
            "(x |> f()) |> g()",
            "x %>% sum() %>% mean()",
            "x |> sum() |> mean()",
            "x %>% sum() %>% mean() %>% sd()",
        ] {
            expect_no_lint(code, "one_call_pipe", None);
        }
    }

    #[test]
    fn test_no_lint_rhs_call_with_arguments() {
        for code in [
            "xgb_spec |> fit_xy(mtcar_mat, mtcars$mpg)",
            "df |> select(x)",
            "df %>% select(x)",
            "df |> dplyr::select(x)",
            "df |> mutate(y = x)",
            "df |> f(y = g())",
            "x %>% f(.)",
            "x |> f(_)",
        ] {
            expect_no_lint(code, "one_call_pipe", None);
        }
    }

    #[test]
    fn test_no_lint_non_pipe_expressions() {
        for code in [
            "x + 1",
            "x %other% sum()",
            "sum(x)",
            "x |> y",
            "x %>% y",
            "x |> 1",
            "x %>% { sum(.) + mean(.) }",
            "x |> f",
            "x %>% identity",
            "x %other% f()",
            "x <- y",
        ] {
            expect_no_lint(code, "one_call_pipe", None);
        }
    }

    #[test]
    fn test_lint_one_call_pipe() {
        assert_snapshot!(
            snapshot_lint("x |> sum()"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | x |> sum()
          | ---------- Avoid using a pipe for an expression with only one function call.
          |
          = help: Replace with `sum(x)`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1:10 %>% sum()"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | 1:10 %>% sum()
          | -------------- Avoid using a pipe for an expression with only one function call.
          |
          = help: Replace with `sum(1:10)`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("(x + 1) |> abs()"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | (x + 1) |> abs()
          | ---------------- Avoid using a pipe for an expression with only one function call.
          |
          = help: Replace with `abs((x + 1))`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %T>% sum()"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | x %T>% sum()
          | ------------ Avoid using a pipe for an expression with only one function call.
          |
          = help: Use a regular function call instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %>% { sum(.) }"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | x %>% { sum(.) }
          | ---------------- Avoid using a pipe for an expression with only one function call.
          |
          = help: Use a regular function call instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %>% .()"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | x %>% .()
          | --------- Avoid using a pipe for an expression with only one function call.
          |
          = help: Use a regular function call instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x |> _()"),
            @"
        warning: one_call_pipe
         --> <test>:1:1
          |
        1 | x |> _()
          | -------- Avoid using a pipe for an expression with only one function call.
          |
          = help: Use a regular function call instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_one_call_pipe() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x |> sum()",
                    "1:10 %>% sum()",
                    "(x + 1) |> abs()",
                    "x |> # preserve comment\n  f()",
                    "x %T>% sum()",
                    "x %<>% sum()",
                    "x %!>% sum()",
                    "x %>% { sum(.) }",
                    "x %>% .()",
                    "x |> _()",
                ],
                "one_call_pipe",
                None,
            )
        );
    }
}
