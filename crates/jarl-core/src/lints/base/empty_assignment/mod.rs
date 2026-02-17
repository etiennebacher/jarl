pub(crate) mod empty_assignment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "empty_assignment", None)
    }

    #[test]
    fn test_lint_empty_assignment() {
        assert_snapshot!(
            snapshot_lint("x <- {}"),
            @r"
        warning: empty_assignment
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
            @r"
        warning: empty_assignment
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
            @r"
        warning: empty_assignment
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
            @r"
        warning: empty_assignment
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
            @r"
        warning: empty_assignment
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
    fn test_no_lint_empty_assignment() {
        expect_no_lint("x <- { 3 + 4 }", "empty_assignment", None);
        expect_no_lint("x = if (x > 1) { 3 + 4 }", "empty_assignment", None);
        expect_no_lint("{ 3 + 4 } -> x", "empty_assignment", None);
        expect_no_lint("x <- function() { }", "empty_assignment", None);
    }
}
