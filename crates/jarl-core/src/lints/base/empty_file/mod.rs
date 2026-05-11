pub(crate) mod empty_file;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "empty_file", None)
    }

    #[test]
    fn test_lint_empty_file() {
        assert_snapshot!(
            snapshot_lint("x <- {}"),
            @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | x <- {}
          | ------- Assign NULL explicitly or, whenever possible, allocate the empty object with the right type and size.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x = { }"),
            @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | x = { }
          | ------- Assign NULL explicitly or, whenever possible, allocate the empty object with the right type and size.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("{ } -> x"),
            @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | { } -> x
          | -------- Assign NULL explicitly or, whenever possible, allocate the empty object with the right type and size.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x <- {\n}"),
            @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | / x <- {
        2 | | }
          | |_- Assign NULL explicitly or, whenever possible, allocate the empty object with the right type and size.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("env$obj <- {}"),
            @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | env$obj <- {}
          | ------------- Assign NULL explicitly or, whenever possible, allocate the empty object with the right type and size.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_empty_file() {
        expect_no_lint("x <- { 3 + 4 }", "empty_file", None);
        expect_no_lint("x = if (x > 1) { 3 + 4 }", "empty_file", None);
        expect_no_lint("{ 3 + 4 } -> x", "empty_file", None);
        expect_no_lint("x <- function() { }", "empty_file", None);
    }
}
