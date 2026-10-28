pub(crate) mod dots_access;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "dots_access", Some("4.1"))
    }

    #[test]
    fn test_no_lint_dots_access() {
        for code in [
            "...names()",
            "...length()",
            "...elt(i)",
            "..1",
            "names(x)",
            "names()",
            "names(list())",
            "names(list(x, ...))",
            "names(list(args = ...))",
            "names(list(..., ))",
            "names(list(...), extra = TRUE)",
            "length(list(...), extra = TRUE)",
            "names(other = list(...))",
            "length(list(x, ...))",
            "length(x =)",
            "stats::names(list(...))",
            "stats::length(list(...))",
            "names(pkg::list(...))",
            "list(x, ...)[[1]]",
            "list(...)[1]",
            "list(...)[[\"name\"]]",
            "list(...)[[(\"name\")]]",
            "list(...)[[c(1, 2)]]",
            "list(...)[[list(1)]]",
            "list(...)[[1:2]]",
            "list(...)[[0]]",
            "list(...)[[0x2L]]",
            "list(...)[[-1]]",
            "list(...)[[NA]]",
            "list(...)[[NULL]]",
            "list(...)[[TRUE]]",
            "list(...)[[Inf]]",
            "list(...)[[1i]]",
            "list(...)[[...]]",
            "list(...)[[1, exact = FALSE]]",
            "list(...)[[i = 1]]",
            "list(...)[[]]",
            "names(list(...)) <- nms",
            "length(list(...)) <<- 2",
            "value -> list(...)[[1]]",
            "value ->> (list(...)[[1]])",
            "names(list(...))[[1]] <- \"name\"",
            "list(...)[[1]][i] <- value",
            "x[[length(list(...)) <- value]] <- value",
            "x |> names(list(...))",
            "x %>% ((length(list(...))))",
            "x %!>% names(list(...))",
            "x %T>% names(list(...))",
            "x %<>% names(list(...))",
            "x %>% list(...)[[1]]",
        ] {
            expect_no_lint(code, "dots_access", Some("4.1"));
        }
    }

    #[test]
    fn test_no_lint_dots_access_when_r_version_is_unsupported() {
        for code in ["names(list(...))", "length(list(...))", "list(...)[[1]]"] {
            expect_no_lint(code, "dots_access", Some("4.0"));
            expect_no_lint(code, "dots_access", None);
        }
    }

    #[test]
    fn test_lint_dots_access() {
        assert_snapshot!(snapshot_lint("names(list(...))"), @"
        warning: dots_access
         --> <test>:1:1
          |
        1 | names(list(...))
          | ---------------- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...names()` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("length(x = list(...))"), @"
        warning: dots_access
         --> <test>:1:1
          |
        1 | length(x = list(...))
          | --------------------- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...length()` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("list(...)[[2L]]"), @"
        warning: dots_access
         --> <test>:1:1
          |
        1 | list(...)[[2L]]
          | --------------- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `..2` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("list(...)[[index]]"), @"
        warning: dots_access
         --> <test>:1:1
          |
        1 | list(...)[[index]]
          | ------------------ Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...elt(index)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("list(...)[[index + 1]]"), @"
        warning: dots_access
         --> <test>:1:1
          |
        1 | list(...)[[index + 1]]
          | ---------------------- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...elt(index + 1)` instead.
        Found 1 error.
        ");
        // Indices in replacement assignments still read from `...`.
        assert_snapshot!(snapshot_lint("x[[length(list(...))]] <- value"), @"
        warning: dots_access
         --> <test>:1:4
          |
        1 | x[[length(list(...))]] <- value
          |    ----------------- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...length()` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("value -> x[names(list(...))]"), @"
        warning: dots_access
         --> <test>:1:12
          |
        1 | value -> x[names(list(...))]
          |            ---------------- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...names()` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("names(\n  # keep this comment\n  list(...)\n)"), @"
        warning: dots_access
         --> <test>:1:1
          |
        1 | / names(
        2 | |   # keep this comment
        3 | |   list(...)
        4 | | )
          | |_- Constructing a list to access `...` is unnecessary.
          |
          = help: Use `...names()` instead.
        Found 1 error.
        ");
    }

    #[test]
    fn test_dots_access_fix() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "f <- function(...) {\n  names(list(...))\n  length(list(...))\n  list(...)[[1]]\n  list(...)[[i]]\n}",
                    "base::names(x = (base::list(...)))",
                    "base::length(\"x\" = list(...))",
                    "length(`x` = list(...))",
                    "(base::list(...))[[((2L))]]",
                    "list(...)[[1.0]]",
                    "list(...)[[1e1]]",
                    "list(...)[[1.5]]",
                    "list(...)[[index + 1]]",
                    "list(...)[[get_index()]]",
                    "x[[length(list(...))]] <- value",
                    "value -> x[names(list(...))]",
                    "x[[list(...)[[i]]]] <<- value",
                    "value ->> x[[list(...)[[2L]]]]",
                    "names(list(...)) %>% print()",
                    "x %>% f(length(list(...)))",
                    "names(\n  list(...)\n)",
                    "# leading comment\nnames(list(...)) # trailing comment",
                    "names(\n  # keep this comment\n  list(...)\n)",
                    "length(list( # keep this too\n  ...))",
                    "list(...)[[ # index comment\n  1]]",
                ],
                "dots_access",
                Some("4.1")
            )
        );
    }
}
