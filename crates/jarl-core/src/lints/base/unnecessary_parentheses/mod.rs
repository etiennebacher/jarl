pub(crate) mod unnecessary_parentheses;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unnecessary_parentheses", None)
    }

    #[test]
    fn test_lint_unnecessary_parentheses() {
        assert_snapshot!(snapshot_lint("((x))"), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | ((x))
          | ----- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("foo(((x)))"), @"
        warning: unnecessary_parentheses
         --> <test>:1:5
          |
        1 | foo(((x)))
          |     ----- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("((x)) + y"), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | ((x)) + y
          | ----- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("if (((x))) y"), @"
        warning: unnecessary_parentheses
         --> <test>:1:5
          |
        1 | if (((x))) y
          |     ----- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("((x + y)) * z"), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | ((x + y)) * z
          | --------- This expression contains an unnecessary pair of parentheses.
          |
          = help: Remove the unnecessary pair of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint(
            "(
  (x)
)",
        ), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | / (
        2 | |   (x)
        3 | | )
          | |_- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint(
            "(
  # explain x
  (x)
)",
        ), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | / (
        2 | |   # explain x
        3 | |   (x)
        4 | | )
          | |_- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        Found 1 error.
        ");
    }

    #[test]
    fn test_reports_once_per_expression() {
        assert_snapshot!(snapshot_lint(
            "((x))
(((x)))
((((x))))",
        ), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | ((x))
          | ----- This expression contains 2 unnecessary pairs of parentheses.
          |
          = help: Remove 2 pairs of parentheses.
        warning: unnecessary_parentheses
         --> <test>:2:1
          |
        2 | (((x)))
          | ------- This expression contains 3 unnecessary pairs of parentheses.
          |
          = help: Remove 3 pairs of parentheses.
        warning: unnecessary_parentheses
         --> <test>:3:1
          |
        3 | ((((x))))
          | --------- This expression contains 4 unnecessary pairs of parentheses.
          |
          = help: Remove 4 pairs of parentheses.
        Found 3 errors.
        ");
    }

    #[test]
    fn test_fix_unnecessary_parentheses() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "((x))",
                    "(((x)))",
                    "((((x))))",
                    "((x + 1))",
                    "foo(((x)))",
                    "((x)) + y",
                    "if (((x))) y",
                    "(
  (x)
)",
                    "((x + y)) * z",
                    "z * ((x + y))",
                    "((x * y)) + z",
                    "-((x + y))",
                ],
                "unnecessary_parentheses",
                None,
            )
        );
    }

    #[test]
    fn test_unnecessary_parentheses_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "(
  # explain x
  (x)
)",
                    "# leading comment
((x))",
                    "((x)) # trailing comment",
                ],
                "unnecessary_parentheses",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_unnecessary_parentheses() {
        expect_no_lint("x", "unnecessary_parentheses", None);
        expect_no_lint("(x)", "unnecessary_parentheses", None);
        expect_no_lint("(x + y) * z", "unnecessary_parentheses", None);
        expect_no_lint("foo(x)", "unnecessary_parentheses", None);
        expect_no_lint("if (x) y", "unnecessary_parentheses", None);
        expect_no_lint("while (x) y", "unnecessary_parentheses", None);
        expect_no_lint("function(x) x", "unnecessary_parentheses", None);
    }
}
