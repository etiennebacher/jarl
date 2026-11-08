pub(crate) mod comparison_negation;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "comparison_negation", None)
    }

    #[test]
    fn test_no_lint_comparison_negation() {
        for code in [
            "!(x == y | y == z)",
            "!(x & y)",
            "!(x + y)",
            "-(x < y)",
            "!any(x > y)",
            "!passes.test[stage == 1]",
            "!!target == 1 ~ 'target'",
            "!!x < y",
            "!!(x < y)",
            "!!!(x == y)",
            "rlang::expr(!!(x < y))",
            "rlang::expr(list(!!!((x < y))))",
        ] {
            expect_no_lint(code, "comparison_negation", None);
        }

        // Ensure that we capture a (very unlikely) scenario where operator precedence
        // changes in Air.
        expect_no_lint("2 * !(1 < 2) + 1", "comparison_negation", None);
    }

    #[test]
    fn test_lint_comparison_negation() {
        assert_snapshot!(snapshot_lint("!(x >= y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x >= y)
          | --------- This negated comparison can be simplified.
          |
          = help: Use `x < y` instead of `!(x >= y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("!(x > y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x > y)
          | -------- This negated comparison can be simplified.
          |
          = help: Use `x <= y` instead of `!(x > y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("!(x <= y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x <= y)
          | --------- This negated comparison can be simplified.
          |
          = help: Use `x > y` instead of `!(x <= y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("!(x < y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x < y)
          | -------- This negated comparison can be simplified.
          |
          = help: Use `x >= y` instead of `!(x < y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("!(x == y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x == y)
          | --------- This negated comparison can be simplified.
          |
          = help: Use `x != y` instead of `!(x == y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("!(x != y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x != y)
          | --------- This negated comparison can be simplified.
          |
          = help: Use `x == y` instead of `!(x != y)`.
        Found 1 error.
        ");
        // Extra parentheses and complex operands appear accurately in the suggestion.
        assert_snapshot!(snapshot_lint("!(((sum(x) < limit)))"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(((sum(x) < limit)))
          | --------------------- This negated comparison can be simplified.
          |
          = help: Use `x >= y` instead of `!(x < y)`.
        Found 1 error.
        ");
        // Comparisons bind more tightly than logical negation, even without parentheses.
        assert_snapshot!(snapshot_lint("!length(x) > 0"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !length(x) > 0
          | -------------- This negated comparison can be simplified.
          |
          = help: Use `x <= y` instead of `!(x > y)`.
        Found 1 error.
        ");
        // The suggested replacement preserves grouping in the surrounding expression.
        assert_snapshot!(snapshot_lint("2 * !(1 < 2)"), @"
        warning: comparison_negation
         --> <test>:1:5
          |
        1 | 2 * !(1 < 2)
          |     -------- This negated comparison can be simplified.
          |
          = help: Use `x >= y` instead of `!(x < y)`.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("TRUE == !(1 < 2)"), @"
        warning: comparison_negation
         --> <test>:1:9
          |
        1 | TRUE == !(1 < 2)
          |         -------- This negated comparison can be simplified.
          |
          = help: Use `x >= y` instead of `!(x < y)`.
        Found 1 error.
        ");
        // Comments inside the expression still allow a diagnostic.
        assert_snapshot!(snapshot_lint("!(x\n # keep this comment\n >= y)"), @"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | / !(x
        2 | |  # keep this comment
        3 | |  >= y)
          | |______- This negated comparison can be simplified.
          |
          = help: Use `x < y` instead of `!(x >= y)`.
        Found 1 error.
        ");
    }

    #[test]
    fn test_fix_comparison_negation() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "!(x >= y)",
                    "!(x > y)",
                    "!(x <= y)",
                    "!(x < y)",
                    "!(x == y)",
                    "!(x != y)",
                    "!(foo(x + 1) != foo(bar(if (x == 1) 2 else 3)))",
                    "!(((sum(x) < limit)))",
                    "!length(x) > 0",
                    // Preserve grouping around arithmetic, unary, and comparison operands.
                    "2 * !(1 < 2)",
                    "2 ^ !(x < y)",
                    "1:!(x < y)",
                    "TRUE %in% !(x < y)",
                    "-!(1 < 2)",
                    "TRUE == !(1 < 2)",
                    "!(x < y) == FALSE",
                    // Existing grouping and lower-precedence operators need no extra parentheses.
                    "2 * (!(x < y))",
                    "if (TRUE && !(bar > foo)) 1",
                    "result <- !(x < y)",
                    "z ~ !(x < y)",
                    // Parentheses separate these bangs; this is ordinary double negation.
                    "!(!(x < y))",
                    "!(总数 < 阈值)",
                    // Keep surrounding comments; skip fixes that would remove internal comments.
                    "# leading comment\n!(x >= y)",
                    "!(x\n # keep this comment\n >= y)",
                    "!(x >= y) # trailing comment",
                ],
                "comparison_negation",
                None,
            )
        );
    }
}
