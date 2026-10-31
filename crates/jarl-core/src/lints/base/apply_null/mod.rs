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
            "lapply(x, function(x) if (x) NULL else NULL)",
            "lapply(x, function(x) { NULL; x })",
            "map(x, function(x) NULL)",
            "other::lapply(x, function(x) NULL)",
            "lapply(x)",
        ] {
            expect_no_lint(code, "apply_null", None);
        }
    }

    #[test]
    fn test_lint_apply_null() {
        for code in [
            "lapply(x, function(x) NULL)",
            "sapply(x, function(x) NULL)",
            "base::lapply(x, function(x) NULL)",
            "base::sapply(x, function(x) NULL)",
            "lapply(X = x, FUN = function(...) NULL)",
            "lapply(x, function(x) { NULL })",
            "lapply(x, function(x) return(NULL))",
            "lapply(x, function(x) { return(NULL) })",
            "sapply(x, function(x) return())",
        ] {
            assert!(
                snapshot_lint(code).contains("warning: apply_null"),
                "expected {code:?} to trigger apply_null"
            );
        }

        assert_snapshot!(
            snapshot_lint("lapply(x, function(x) NULL)"),
            @r###"
        warning: apply_null
         --> <test>:1:1
          |
        1 | lapply(x, function(x) NULL)
          | --------------------------- This call iterates over its input to create a list of `NULL` values.
          |
          = help: Use `vector("list", length(x))` to allocate the list directly.
        Found 1 error.
        "###
        );
    }
}
