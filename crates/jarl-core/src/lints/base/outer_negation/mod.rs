pub(crate) mod outer_negation;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "outer_negation", None)
    }

    #[test]
    fn test_no_lint_outer_negation() {
        for code in [
            "x <- any(y)",
            "y <- all(z)",
            "any()",
            "all(x)",
            "!any(x)",
            // Extended uses of any() and all() are outside this rule's pattern.
            "any(!a & b)",
            "all(a | !b)",
            "any(a, b)",
            "any(!a, b)",
            "all(b, c)",
            "all(a, !b)",
            "any(!a, !b)",
            "any(a, !b, na.rm = TRUE)",
            "any(a, !b, 'na.rm' = TRUE)",
            "any(na.rm = !FALSE)",
            "all('na.rm' = !TRUE)",
            "any(!x, na.rm = TRUE, na.rm = FALSE)",
            "any(na.rm = TRUE, na.rm = !x)",
            "all('na.rm' = TRUE, `na.rm` = !x)",
            "any(!!a)",
            "any(!!!a)",
            "all(!!!x)",
            "any((!!x))",
            "any(-a)",
            "any(+a)",
            "all(~x)",
            "FALSE |> any(!FALSE)",
            "x |> base::all(!y)",
            "x %>% any(!y)",
            "x %!>% all(!y)",
            "x %T>% any(!y)",
            "x %<>% all(!y)",
            "x %>% (any(!y))",
            "x %>% any(!.)",
            "pkg::any(!x)",
            "pkg::all(!x)",
            r#""pkg"::any(!x)"#,
            "any(!x, )",
            "any(!x, na.rm = )",
            "any(, !x)",
            "any(!x, ...)",
        ] {
            expect_no_lint(code, "outer_negation", None);
        }
    }

    #[test]
    fn test_lint_outer_negation() {
        assert_snapshot!(snapshot_lint("any(!x)"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | any(!x)
          | ------- `any(!...)` may be hard to read.
          |
          = help: Use `!all(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("all(!x)"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | all(!x)
          | ------- `all(!...)` may be hard to read.
          |
          = help: Use `!any(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("base::any(((!x)), na.rm = TRUE)"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | base::any(((!x)), na.rm = TRUE)
          | ------------------------------- `any(!...)` may be hard to read.
          |
          = help: Use `!all(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("any(na.rm = TRUE, result = !f(x, y))"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | any(na.rm = TRUE, result = !f(x, y))
          | ------------------------------------ `any(!...)` may be hard to read.
          |
          = help: Use `!all(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("!any(!x)"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | !any(!x)
          | -------- `!any(!...)` may be hard to read.
          |
          = help: Use `all(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("any(!-x)"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | any(!-x)
          | -------- `any(!...)` may be hard to read.
          |
          = help: Use `!all(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("any(\n  # Keep this comment.\n  !x\n)"), @"
        warning: outer_negation
         --> <test>:1:1
          |
        1 | / any(
        2 | |   # Keep this comment.
        3 | |   !x
        4 | | )
          | |_- `any(!...)` may be hard to read.
          |
          = help: Use `!all(...)` instead.
        Found 1 error.
        ");
    }

    #[test]
    fn test_outer_negation_fix() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "any(!x)",
                    "all(!f(x, y))",
                    "any(!f(all(!x)))",
                    "all(!f(any(!x)))",
                    "base::any(((!x)), na.rm = TRUE)",
                    "any((!((x + y))))",
                    "all(!((x = TRUE)))",
                    "f <- function(...) any(!((...)))",
                    "all('na.rm' = FALSE, result = !f(x, y))",
                    "any(!x, `na.rm` = remove_na())",
                    "any(na = !x)",
                    "any(\n  !x,\n  na.rm = TRUE\n)",
                    "any(!-x)",
                    "all(!+x)",
                    "!any(!x)",
                    "!!any(!x)",
                    "!(base::all(!x))",
                    "! any(!x, na.rm = TRUE)",
                    "if (!any(!x)) 1",
                    "!any(!x) + 2",
                    "any(!FALSE) + 2",
                    "2 * all(!TRUE)",
                    "any(!x) == y",
                    "any(!x) |> as.integer()",
                    "any(!x)[1]",
                    "(any(!x)) + 2",
                    "result <- any(!x)",
                    "f(any(!x))",
                    "# Leading comment\nany(!x)",
                    "all(!x) # Trailing comment",
                    "any(\n  # Keep this comment.\n  !x\n)",
                    "all(!(x + # Keep this comment too.\n  y))",
                ],
                "outer_negation"
            )
        );
    }
}
