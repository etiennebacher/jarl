pub(crate) mod unnecessary_parenthesis;

#[cfg(test)]
mod tests {
    use crate::rule_set::Rule;
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unnecessary_parenthesis", None)
    }

    #[test]
    fn test_lint_unnecessary_parenthesis() {
        assert_snapshot!(snapshot_lint("((x))"), @"
        warning: unnecessary_parenthesis
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
                !check_code(code, "unnecessary_parenthesis", None).is_empty(),
                "Expected a lint for: {code}",
            );
        }
    }

    #[test]
    fn test_reports_each_redundant_level() {
        let diagnostics = check_code("(((x)))", "unnecessary_parenthesis", None);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(usize::from(diagnostics[0].range.start()), 0);
        assert_eq!(usize::from(diagnostics[0].range.end()), 7);
        assert_eq!(usize::from(diagnostics[1].range.start()), 1);
        assert_eq!(usize::from(diagnostics[1].range.end()), 6);
    }

    #[test]
    fn test_no_lint_unnecessary_parenthesis() {
        for code in [
            "x",
            "(x)",
            "(x + y) * z",
            "foo(x)",
            "if (x) y",
            "while (x) y",
            "function(x) x",
        ] {
            expect_no_lint(code, "unnecessary_parenthesis", None);
        }
    }

    #[test]
    fn test_unnecessary_parenthesis_metadata() {
        assert!(Rule::UnnecessaryParenthesis.is_enabled_by_default());
        assert!(Rule::UnnecessaryParenthesis.has_no_fix());
    }
}
