pub(crate) mod null_list;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "null_list", None)
    }

    #[test]
    fn test_no_lint_null_list() {
        for code in [
            // Function doesn't (only) return `NULL`
            "lapply(x, function(x) x)",
            "sapply(x, function(x) x)",
            "lapply(x, function(x) { message(x); NULL })",
            "lapply(x, function(x) { message(x); return(NULL) })",
            "lapply(x, function(x) if (x) NULL else NULL)",
            "lapply(x, function(x) { NULL; x })",
            "lapply(x, function(x) foo(NULL))",
            "lapply(x, function(x) return(x))",
            "lapply(x, function(x) return(NULL, x))",
            "purrr::map(x, function(x) x)",
            "purrr::map(x, ~x)",
            "purrr::map(x, ~ { message(.x); NULL })",
            // Bare `map()` without `library(purrr)`
            "map(x, function(x) NULL)",
            "map(x, ~NULL)",
            // Doesn't handle package namespaces
            "library(purrr); map(x, function(x) NULL)",
            "library(maps); map(x, function(x) NULL)",
            // Not one of the targeted functions
            "purrr::map_chr(x, function(x) NULL)",
            "other::map(x, function(x) NULL)",
            "other::lapply(x, function(x) NULL)",
            // Formulas are only lambdas in `purrr::map()`
            "lapply(x, ~NULL)",
            // No anonymous function to inspect
            "lapply(x)",
            "lapply(x, identity)",
        ] {
            expect_no_lint(code, "null_list", None);
        }
    }

    #[test]
    fn test_lint_null_list() {
        assert_snapshot!(snapshot_lint("lapply(x, function(x) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | lapply(x, function(x) NULL)
          | --------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("sapply(x, function(x) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | sapply(x, function(x) NULL)
          | --------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("base::lapply(x, function(x) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | base::lapply(x, function(x) NULL)
          | --------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("base::sapply(x, function(x) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | base::sapply(x, function(x) NULL)
          | --------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(X = x, FUN = function(...) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | lapply(X = x, FUN = function(...) NULL)
          | --------------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) { NULL })"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | lapply(x, function(x) { NULL })
          | ------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) return(NULL))"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | lapply(x, function(x) return(NULL))
          | ----------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) { return(NULL) })"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | lapply(x, function(x) { return(NULL) })
          | --------------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("sapply(x, function(x) return())"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | sapply(x, function(x) return())
          | ------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("sapply(x, function(...) return(value = NULL))"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | sapply(x, function(...) return(value = NULL))
          | --------------------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) {})"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | lapply(x, function(x) {})
          | ------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("purrr::map(x, function(x) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | purrr::map(x, function(x) NULL)
          | ------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint(r"purrr::map(x, \(x) NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | purrr::map(x, \(x) NULL)
          | ------------------------ This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("purrr::map(x, ~NULL)"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | purrr::map(x, ~NULL)
          | -------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("purrr::map(x, ~ { NULL })"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | purrr::map(x, ~ { NULL })
          | ------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("purrr::map(.x = x, .f = function(x) return(NULL))"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | purrr::map(.x = x, .f = function(x) return(NULL))
          | ------------------------------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("purrr::map(x, function(x) {})"), @r#"
        warning: null_list
         --> <test>:1:1
          |
        1 | purrr::map(x, function(x) {})
          | ----------------------------- This way of creating an empty list is inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly. Names of `x` are not kept.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("library(purrr); map(x, ~NULL)"), @"All checks passed!");
    }

    #[test]
    fn test_fix_null_list() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "lapply(x, function(x) NULL)",
                    "sapply(x, function(x) NULL)",
                    "base::lapply(x, function(x) { return(NULL) })",
                    "lapply(FUN = function(...) NULL, X = foo$bar)",
                    "lapply(names(x), function(x) NULL)",
                    "purrr::map(x, ~NULL)",
                    r"purrr::map(.f = \(x) NULL, .x = 1:10)",
                    "y <- lapply(\n  x,\n  function(x) NULL\n)",
                ],
                "null_list",
            )
        );
    }

    #[test]
    fn test_null_list_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "lapply(x, function(x) NULL # comment\n)",
                    "lapply(\n  # comment\n  x,\n  function(x) NULL\n)",
                ],
                "null_list",
            )
        );
    }
}
