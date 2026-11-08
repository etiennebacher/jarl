pub(crate) mod terminal_close;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "terminal_close", None)
    }

    #[test]
    fn test_no_lint_terminal_close() {
        let rule = "terminal_close";
        expect_no_lint(
            "foo <- function(bar) {
              tmp <- tempfile()
              on.exit(close(tmp))
              writeLines(bar, tmp)
              return(invisible())
            }",
            rule,
            None,
        );
        expect_no_lint(
            "foo <- function(bar) {
              close <- bar + 1
              return(close)
            }",
            rule,
            None,
        );
        expect_no_lint(
            "foo <- function(bar) {
              close <- bar + 1
              close
            }",
            rule,
            None,
        );
        expect_no_lint(
            "foo <- \\(bar) {
              close <- bar + 1
              return(close)
            }",
            rule,
            None,
        );
        // Not in terminal position
        expect_no_lint(
            "foo <- function(x) {
          close(x)
          1
        }",
            rule,
            None,
        );
        expect_no_lint("close(x)", rule, None);
        expect_no_lint("foo <- function(x) close(x)", rule, None);
    }

    #[test]
    fn test_lint_terminal_close() {
        assert_snapshot!(
            snapshot_lint("
foo <- function(bar) {
  tmp <- tempfile()
  writeLines(bar, tmp)
  return(close(tmp))
}"),
            @"
        warning: terminal_close
         --> <test>:5:3
          |
        5 |   return(close(tmp))
          |   ------------------ `close()` is the last call of the function, so the connection stays open on errors or early returns.
          |
          = help: Use `on.exit(close(x))` right after opening the connection.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
foo <- function(bar) {
  tmp <- tempfile()
  writeLines(bar, tmp)
  close(tmp)
}"),
            @"
        warning: terminal_close
         --> <test>:5:3
          |
        5 |   close(tmp)
          |   ---------- `close()` is the last call of the function, so the connection stays open on errors or early returns.
          |
          = help: Use `on.exit(close(x))` right after opening the connection.
        Found 1 error.
        "
        );
        // Only the last call is reported
        assert_snapshot!(
            snapshot_lint("
foo <- function(bar) {
  close(tmp1)
  close(tmp2)
}"),
            @"
        warning: terminal_close
         --> <test>:4:3
          |
        4 |   close(tmp2)
          |   ----------- `close()` is the last call of the function, so the connection stays open on errors or early returns.
          |
          = help: Use `on.exit(close(x))` right after opening the connection.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(r"
{
  foo <- function() {
    close(a)
  }
  bar <- \() {
    close(b)
  }
}"),
            @"
        warning: terminal_close
         --> <test>:4:5
          |
        4 |     close(a)
          |     -------- `close()` is the last call of the function, so the connection stays open on errors or early returns.
          |
          = help: Use `on.exit(close(x))` right after opening the connection.
        warning: terminal_close
         --> <test>:7:5
          |
        7 |     close(b)
          |     -------- `close()` is the last call of the function, so the connection stays open on errors or early returns.
          |
          = help: Use `on.exit(close(x))` right after opening the connection.
        Found 2 errors.
        "
        );
    }
}
