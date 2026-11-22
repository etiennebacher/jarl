pub(crate) mod paste;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "paste", None)
    }

    #[test]
    fn test_lint_paste_empty_sep() {
        assert_snapshot!(snapshot_lint("paste(sep = '', 'a', 'b')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(sep = '', 'a', 'b')
          | ------------------------- `paste(..., sep = "")` can be simplified.
          |
          = help: Use `paste0(...)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("paste('a', 'b', sep = '')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste('a', 'b', sep = '')
          | ------------------------- `paste(..., sep = "")` can be simplified.
          |
          = help: Use `paste0(...)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("paste('a', 'b', expression(2), sep = '')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste('a', 'b', expression(2), sep = '')
          | ---------------------------------------- `paste(..., sep = "")` can be simplified.
          |
          = help: Use `paste0(...)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint(r#"paste(a, b, sep = R"---[]---")"#), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(a, b, sep = R"---[]---")
          | ------------------------------ `paste(..., sep = "")` can be simplified.
          |
          = help: Use `paste0(...)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("base::paste(a, b, sep = '')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | base::paste(a, b, sep = '')
          | --------------------------- `paste(..., sep = "")` can be simplified.
          |
          = help: Use `paste0(...)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("x |> paste(y, sep = '')"), @r#"
        warning: paste
         --> <test>:1:6
          |
        1 | x |> paste(y, sep = '')
          |      ------------------ `paste(..., sep = "")` can be simplified.
          |
          = help: Use `paste0(...)` instead.
        Found 1 error.
        "#);
    }

    #[test]
    fn test_no_lint_paste_empty_sep() {
        expect_no_lint("paste('a', 'b', 'c')", "paste", None);
        expect_no_lint("paste('a', 'b', 'c', sep = ',')", "paste", None);
        expect_no_lint("paste('a', 'b', collapse = '')", "paste", None);
        expect_no_lint("cat(paste('a', 'b'), sep = '')", "paste", None);
        expect_no_lint("sep <- ''; paste('a', sep)", "paste", None);
        expect_no_lint("paste(sep = ',', '', 'a')", "paste", None);
        expect_no_lint("paste0('a', 'b', 'c')", "paste", None);
        expect_no_lint("paste(a, b, sep = R'(xyz)')", "paste", None);
        expect_no_lint("paste(a, b, sep = )", "paste", None);
        expect_no_lint("foo::paste(a, b, sep = '')", "paste", None);
        // Inside `expression()`, `paste()` is plotmath's juxtaposition (see `?plotmath`).
        expect_no_lint("expression(paste('a', 'b', sep = ''))", "paste", None);
        expect_no_lint("expression(italic(paste('a', sep = '')))", "paste", None);
    }

    #[test]
    fn test_lint_paste_to_string() {
        assert_snapshot!(snapshot_lint("paste(collapse = ', ', x)"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(collapse = ', ', x)
          | ------------------------- `paste(x, collapse = ", ")` can be simplified.
          |
          = help: Use `toString(x)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint(r#"paste(c(a, b, c), collapse = R"-{, }-")"#), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(c(a, b, c), collapse = R"-{, }-")
          | --------------------------------------- `paste(x, collapse = ", ")` can be simplified.
          |
          = help: Use `toString(x)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("paste0(foo(x), collapse = ', ')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste0(foo(x), collapse = ', ')
          | ------------------------------- `paste0(x, collapse = ", ")` can be simplified.
          |
          = help: Use `toString(x)` instead.
        Found 1 error.
        "#);
    }

    #[test]
    fn test_no_lint_paste_to_string() {
        expect_no_lint("paste(x, sep = ', ')", "paste", None);
        expect_no_lint("paste(x, collapse = ',')", "paste", None);
        expect_no_lint("paste(foo(x), collapse = '/')", "paste", None);
        expect_no_lint(
            "collapse <- ', '; paste(x, collapse = collapse)",
            "paste",
            None,
        );
        // `sep` is used, so this is not a trivial swap.
        expect_no_lint("paste(x, y, sep = '.', collapse = ', ')", "paste", None);
        // Several inputs are first combined with the default `sep`.
        expect_no_lint("paste(x, y, collapse = ', ')", "paste", None);
        expect_no_lint("paste0(x, y, collapse = ', ')", "paste", None);
        expect_no_lint("paste(..., collapse = ', ')", "paste", None);
        expect_no_lint("x |> paste(y, collapse = ', ')", "paste", None);
        expect_no_lint("x %>% paste(y, collapse = ', ')", "paste", None);
        expect_no_lint("toString(x)", "paste", None);
        // Only an exact match of ", ".
        expect_no_lint(r#"paste(x, collapse = ", \n")"#, "paste", None);
        expect_no_lint("paste(x, collapse = R'(,,)')", "paste", None);
    }

    #[test]
    fn test_lint_paste_strrep() {
        assert_snapshot!(snapshot_lint("paste(rep('#', width), collapse='')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(rep('#', width), collapse='')
          | ----------------------------------- `paste(rep(x, times), collapse = "")` can be simplified.
          |
          = help: Use `strrep(x, times)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("paste0(rep('*', 20L), collapse='')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste0(rep('*', 20L), collapse='')
          | ---------------------------------- `paste0(rep(x, times), collapse = "")` can be simplified.
          |
          = help: Use `strrep(x, times)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("paste(rep('#', times = n), collapse = '')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(rep('#', times = n), collapse = '')
          | ----------------------------------------- `paste(rep(x, times), collapse = "")` can be simplified.
          |
          = help: Use `strrep(x, times)` instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("paste(base::rep('#', n), collapse = '')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(base::rep('#', n), collapse = '')
          | --------------------------------------- `paste(rep(x, times), collapse = "")` can be simplified.
          |
          = help: Use `strrep(x, times)` instead.
        Found 1 error.
        "#);
        // Reported but not fixed: `strrep()` has no `each` argument.
        assert_snapshot!(snapshot_lint("paste(rep('a', each = 2), collapse = '')"), @r#"
        warning: paste
         --> <test>:1:1
          |
        1 | paste(rep('a', each = 2), collapse = '')
          | ---------------------------------------- `paste(rep(x, times), collapse = "")` can be simplified.
          |
          = help: Use `strrep(x, times)` instead.
        Found 1 error.
        "#);
    }

    #[test]
    fn test_no_lint_paste_strrep() {
        expect_no_lint("paste(x, collapse = '')", "paste", None);
        expect_no_lint("paste(rep('*', 10), collapse = '+')", "paste", None);
        expect_no_lint("paste(rep(c('a', 'b'), 2), collapse = '')", "paste", None);
        expect_no_lint("paste0(rep('a', 2), 'b', collapse = '')", "paste", None);
        expect_no_lint("paste(rep('*', 10))", "paste", None);
        expect_no_lint(
            "paste(rep('*', 10), rep('x', 10), collapse = '')",
            "paste",
            None,
        );
        expect_no_lint("paste(foo::rep('*', 10), collapse = '')", "paste", None);
        expect_no_lint("x |> paste(rep('*', 10), collapse = '')", "paste", None);
    }

    #[test]
    fn test_paste_fix() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "paste('a', 'b', sep = '')",
                    "paste(sep = '', a, b, collapse = ',')",
                    "base::paste(a, b, sep = '')",
                    "x |> paste(y, sep = '')",
                    "paste(x, collapse = ', ')",
                    "paste0(foo(x), collapse = ', ')",
                    "paste(rep('#', width), collapse = '')",
                    "paste0(rep('*', times = 20L), collapse = '')",
                ],
                "paste",
                None,
            )
        );
    }

    #[test]
    fn test_paste_no_fix() {
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(
                vec![
                    "paste(rep('a', each = 2), collapse = '')",
                    "paste(\n  'a', # comment\n  'b',\n  sep = ''\n)",
                ],
                "paste",
                None,
            )
        );
    }
}
