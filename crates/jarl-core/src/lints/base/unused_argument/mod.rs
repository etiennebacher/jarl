pub(crate) mod unused_argument;
#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unused_argument", None)
    }

    // ── No-lint cases ────────────────────────────────────────────────

    #[test]
    fn test_no_lint_used_parameter() {
        expect_no_lint("f <- function(x) x + 1", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_used_parameter_nested() {
        expect_no_lint(
            "f <- function(x) {\n  y <- x + 1\n  y\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_used_as_default_for_other_param() {
        // `x` is used by the default value of `y`.
        expect_no_lint("f <- function(x, y = x) y", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_dots_parameter() {
        expect_no_lint("f <- function(...) 1", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_used_in_nested_closure() {
        expect_no_lint("f <- function(x) function() x", "unused_argument", None);
    }

    #[test]
    fn test_no_lint_s3_generic_inline() {
        // S3 generic: the body is `UseMethod(...)`. Args are forwarded to the
        // dispatched method, not read locally.
        expect_no_lint(
            "print <- function(x, ...) UseMethod(\"print\")",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_s3_generic_braced() {
        // Same with a braced body — possibly with arg-evaluation calls first.
        expect_no_lint(
            "summary <- function(object, ...) {\n  force(object)\n  UseMethod(\"summary\")\n}",
            "unused_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_s4_generic() {
        // S4 generic: dispatch via `standardGeneric`.
        expect_no_lint(
            "show <- function(object) standardGeneric(\"show\")",
            "unused_argument",
            None,
        );
    }

    // ── Lint cases ───────────────────────────────────────────────────

    #[test]
    fn test_lint_simple_unused() {
        assert_snapshot!(
            snapshot_lint("f <- function(x, y) x"),
            @r"
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(x, y) x
          |                  - Argument `y` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_multiple_unused() {
        assert_snapshot!(
            snapshot_lint("f <- function(a, b, c) 1"),
            @r"
        warning: unused_argument
         --> <test>:1:15
          |
        1 | f <- function(a, b, c) 1
          |               - Argument `a` is defined but never used.
          |
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(a, b, c) 1
          |                  - Argument `b` is defined but never used.
          |
        warning: unused_argument
         --> <test>:1:21
          |
        1 | f <- function(a, b, c) 1
          |                     - Argument `c` is defined but never used.
          |
        Found 3 errors.
        "
        );
    }

    #[test]
    fn test_lint_unused_with_default() {
        assert_snapshot!(
            snapshot_lint("f <- function(x, n = 10) x"),
            @r"
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(x, n = 10) x
          |                  - Argument `n` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_unused_in_nested_function() {
        // `b` in the outer function is unused.
        assert_snapshot!(
            snapshot_lint("f <- function(a, b) {\n  function(c) a + c\n}"),
            @r"
        warning: unused_argument
         --> <test>:1:18
          |
        1 | f <- function(a, b) {
          |                  - Argument `b` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_anonymous_function() {
        // Anonymous functions get the same treatment — params are reported.
        // This documents current behaviour: the lambda below has both x and y
        // unused, and we expect them flagged. If we ever want to silence
        // these, see `should_skip_function`.
        assert_snapshot!(
            snapshot_lint("lapply(1:3, function(x, y) 1)"),
            @r"
        warning: unused_argument
         --> <test>:1:22
          |
        1 | lapply(1:3, function(x, y) 1)
          |                      - Argument `x` is defined but never used.
          |
        warning: unused_argument
         --> <test>:1:25
          |
        1 | lapply(1:3, function(x, y) 1)
          |                         - Argument `y` is defined but never used.
          |
        Found 2 errors.
        "
        );
    }
}
