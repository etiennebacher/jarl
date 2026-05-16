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
        // Case 1: zero-byte file
        assert_snapshot!(snapshot_lint(""), @"
        warning: empty_file
        --> <test>:1:1
         |
         |
         = help: Consider deleting the file
        Found 1 error.
        ");

        // Case 2: whitespace only
        assert_snapshot!(snapshot_lint("   \n\n   \n"), @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | ...
         -| This file is empty or only contains comments.
          |
          = help: Consider deleting the file
        Found 1 error.
        ");

        // Case 3: comments only (loose definition)
        assert_snapshot!(snapshot_lint("# just a comment"), @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | # just a comment
          | - This file is empty or only contains comments.
          |
          = help: Consider deleting the file
        Found 1 error.
        ");

        // Case 4: roxygen-style comments only
        assert_snapshot!(snapshot_lint("#' @keywords internal"), @"
        warning: empty_file
         --> <test>:1:1
          |
        1 | #' @keywords internal
          | - This file is empty or only contains comments.
          |
          = help: Consider deleting the file
        Found 1 error.
        ");

        // Case 5: mixed whitespace + comments
        assert_snapshot!(snapshot_lint("\n  # a note\n\n"), @"
        warning: empty_file
        --> <test>:1:1
         |
         |
         = help: Consider deleting the file
        Found 1 error.
        ");
    }

    #[test]
    fn test_no_lint_empty_file() {
        // Any expression means the file is not empty
        expect_no_lint("x <- 1", "empty_file", None);

        // Comments alongside real code don't count as empty
        expect_no_lint("# header\nx <- 1", "empty_file", None);

        // Even a bare literal is content
        expect_no_lint("NULL", "empty_file", None);

        // Whitespace around a single expression is fine
        expect_no_lint("\n\n  x <- 1  \n\n", "empty_file", None);
    }
}
