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
          | ----- This expression contains unnecessary parentheses.
          |
          = help: Remove one pair of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("foo(((x)))"), @"
        warning: unnecessary_parentheses
         --> <test>:1:5
          |
        1 | foo(((x)))
          |     ----- This expression contains unnecessary parentheses.
          |
          = help: Remove one pair of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("((x)) + y"), @"
        warning: unnecessary_parentheses
         --> <test>:1:1
          |
        1 | ((x)) + y
          | ----- This expression contains unnecessary parentheses.
          |
          = help: Remove one pair of parentheses.
        Found 1 error.
        ");

        assert_snapshot!(snapshot_lint("if (((x))) y"), @"
        warning: unnecessary_parentheses
         --> <test>:1:5
          |
        1 | if (((x))) y
          |     ----- This expression contains unnecessary parentheses.
          |
          = help: Remove one pair of parentheses.
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
          | |_- This expression contains unnecessary parentheses.
          |
          = help: Remove one pair of parentheses.
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
          | |_- This expression contains unnecessary parentheses.
          |
          = help: Remove one pair of parentheses.
        Found 1 error.
        ");
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
