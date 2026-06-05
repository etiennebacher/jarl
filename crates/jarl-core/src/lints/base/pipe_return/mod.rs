pub(crate) mod pipe_return;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "pipe_return", None)
    }

    #[test]
    fn test_no_lint_pipe_return() {
        // `return()` wrapping the whole pipe is fine.
        expect_no_lint("return(x %>% sum())", "pipe_return", None);

        // No `return()` on the right-hand side.
        expect_no_lint("x %>% sum()", "pipe_return", None);
        expect_no_lint("x %>% sum() %>% mean()", "pipe_return", None);

        // The native pipe is not considered (it would be a syntax error).
        expect_no_lint("x |> sum()", "pipe_return", None);

        // A plain `return()` call is fine.
        expect_no_lint("return(sum(x))", "pipe_return", None);
    }

    #[test]
    fn test_lint_pipe_return() {
        assert_snapshot!(
            snapshot_lint("x %>% return()"),
            @"
        warning: pipe_return
         --> <test>:1:7
          |
        1 | x %>% return()
          |       -------- Using `return()` after `%>%` doesn't actually return the output, which can create misleading results.
          |
          = help: Either wrap the pipe in `return()` instead, or store the output in an intermediate object and use `return()` on it, e.g. `out <- x %>% sum(); return(out)`.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("x %>% sum() %>% return()"),
            @"
        warning: pipe_return
         --> <test>:1:17
          |
        1 | x %>% sum() %>% return()
          |                 -------- Using `return()` after `%>%` doesn't actually return the output, which can create misleading results.
          |
          = help: Either wrap the pipe in `return()` instead, or store the output in an intermediate object and use `return()` on it, e.g. `out <- x %>% sum(); return(out)`.
        Found 1 error.
        "
        );

        // The `return()` can also carry an explicit argument.
        assert_snapshot!(
            snapshot_lint("x %>% return(y)"),
            @"
        warning: pipe_return
         --> <test>:1:7
          |
        1 | x %>% return(y)
          |       --------- Using `return()` after `%>%` doesn't actually return the output, which can create misleading results.
          |
          = help: Either wrap the pipe in `return()` instead, or store the output in an intermediate object and use `return()` on it, e.g. `out <- x %>% sum(); return(out)`.
        Found 1 error.
        "
        );
    }
}
