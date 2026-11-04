pub(crate) mod apply_null;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "apply_null", None)
    }

    #[test]
    fn test_no_lint_apply_null() {
        for code in [
            "lapply(x, function(x) x)",
            "sapply(x, function(x) x)",
            "lapply(x, function(x) { message(x); NULL })",
            "lapply(x, function(x) { message(x); return(NULL) })",
            "lapply(x, function(x) if (x) NULL else NULL)",
            "lapply(x, function(x) { NULL; x })",
            "lapply(x, function(x) foo(NULL))",
            "lapply(x, function(x) return(x))",
            "lapply(x, function(x) return(NULL, x))",
            "map(x, function(x) NULL)",
            "other::lapply(x, function(x) NULL)",
            "lapply(x)",
            "lapply(x, identity)",
        ] {
            expect_no_lint(code, "apply_null", None);
        }
    }

    #[test]
    fn test_lint_apply_null() {
        assert_snapshot!(snapshot_lint("lapply(x, function(x) NULL)"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(x, function(x) NULL)
          | --------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("sapply(x, function(x) NULL)"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | sapply(x, function(x) NULL)
          | --------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("base::lapply(x, function(x) NULL)"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | base::lapply(x, function(x) NULL)
          | --------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("base::sapply(x, function(x) NULL)"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | base::sapply(x, function(x) NULL)
          | --------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(X = x, FUN = function(...) NULL)"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(X = x, FUN = function(...) NULL)
          | --------------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) { NULL })"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(x, function(x) { NULL })
          | ------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) return(NULL))"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(x, function(x) return(NULL))
          | ----------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) { return(NULL) })"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(x, function(x) { return(NULL) })
          | --------------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("sapply(x, function(x) return())"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | sapply(x, function(x) return())
          | ------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("sapply(x, function(...) return(value = NULL))"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | sapply(x, function(...) return(value = NULL))
          | --------------------------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("lapply(x, function(x) {})"), @r#"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(x, function(x) {})
          | ------------------------- This way of creating an empty list can be inefficient.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "#);
    }
}
