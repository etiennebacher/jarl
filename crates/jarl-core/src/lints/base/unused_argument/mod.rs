pub(crate) mod unused_argument;
#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unused_argument", None)
    }

    // ---------------------------------------------------------------
    // No-lint cases
    // ---------------------------------------------------------------

    #[test]
    fn test_no_lint_all_params_used() {
        expect_no_lint(
            "f <- function(x, y) {\n  x + y\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_single_param_used() {
        expect_no_lint("f <- function(x) {\n  x + 1\n}", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_param_used_in_nested_call() {
        expect_no_lint(
            "f <- function(x) {\n  print(mean(x))\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_param_in_condition() {
        expect_no_lint(
            "f <- function(x) {\n  if (x) 1 else 2\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_dots() {
        expect_no_lint(
            "f <- function(x, ...) {\n  x + 1\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_param_returned_implicitly() {
        expect_no_lint("f <- function(x) {\n  x\n}", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_use_method() {
        // S3 generics call UseMethod() — params are dispatched, not used directly
        expect_no_lint(
            "print.myclass <- function(x, ...) {\n  UseMethod(\"print\")\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_match_call() {
        // match.call() captures all args dynamically
        expect_no_lint(
            "f <- function(x, y, z) {\n  mc <- match.call()\n  mc\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_no_params() {
        expect_no_lint("f <- function() {\n  1\n}", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_lambda() {
        expect_no_lint("f <- \\(x) x + 1", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_trycatch_handler() {
        expect_no_lint(
            "tryCatch(expr, error = function(e) { 'oops' })",
            "unused_argument",
            None,
        );
        expect_no_lint(
            "tryCatch(expr, warning = function(w) { 'warn' })",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_try_fetch_handler() {
        expect_no_lint(
            "try_fetch(expr, error = function(e) { 'oops' })",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_with_calling_handlers() {
        expect_no_lint(
            "withCallingHandlers(expr, message = function(m) { 'msg' })",
            "unused_argument",
            None,
        );
    }

    // ---------------------------------------------------------------
    // Lint cases
    // ---------------------------------------------------------------

    #[test]
    fn test_lint_simple_unused_param() {
        assert_snapshot!(
            snapshot_lint("f <- function(x, y) {\n  x + 1\n}"),
            @r"
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(x, y) {
          |                  - Argument `y` is defined in the function but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_all_params_unused() {
        assert_snapshot!(
            snapshot_lint("f <- function(x, y) {\n  1 + 1\n}"),
            @r"
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(x, y) {
          |                  - Argument `y` is defined in the function but never used.
          |
        warning: unused_argument
         --> <test>:1:15
          |
        1 | f <- function(x, y) {
          |               - Argument `x` is defined in the function but never used.
          |
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_lint_one_of_many_unused() {
        assert_snapshot!(
            snapshot_lint("f <- function(a, b, c) {\n  a + c\n}"),
            @r"
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(a, b, c) {
          |                  - Argument `b` is defined in the function but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_param_reassigned_using_itself() {
        // .cols is used in enquo(.cols) before being reassigned
        expect_no_lint(
            "across <- function(.cols) {\n  .cols <- enquo(.cols)\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_lint_unused_in_lambda() {
        assert_snapshot!(
            snapshot_lint("f <- \\(x, y) x + 1"),
            @r"
        warning: unused_argument
         --> <test>:1:11
          |
        1 | f <- \(x, y) x + 1
          |           - Argument `y` is defined in the function but never used.
          |
        Found 1 error.
        "
        );
    }
}
