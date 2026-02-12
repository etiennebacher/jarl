pub(crate) mod internal_function;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "internal_function", None)
    }

    #[test]
    fn test_no_lint_internal_function() {
        expect_no_lint("foo::bar()", "internal_function", None);
        expect_no_lint("foo::bar", "internal_function", None);
    }

    #[test]
    fn test_lint_internal_function() {
        assert_snapshot!(
            snapshot_lint("foo:::bar()"),
            @r"
        warning: internal_function
         --> <test>:1:1
          |
        1 | foo:::bar()
          | --------- Accessing a package's internal function with `:::` is likely to break in the future.
          |
          = help: Use public functions via `::` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("foo:::bar"),
            @r"
        warning: internal_function
         --> <test>:1:1
          |
        1 | foo:::bar
          | --------- Accessing a package's internal function with `:::` is likely to break in the future.
          |
          = help: Use public functions via `::` instead.
        Found 1 error.
        "
        );
    }
}
