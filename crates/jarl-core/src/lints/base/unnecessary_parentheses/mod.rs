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

        for code in [
            "foo(((x)))",
            "((x)) + y",
            "if (((x))) y",
            "(\n  (x)\n)",
            "(\n  # explain x\n  (x)\n)",
        ] {
            assert!(
                !check_code(code, "unnecessary_parentheses", None).is_empty(),
                "Expected a lint for: {code}",
            );
        }
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
