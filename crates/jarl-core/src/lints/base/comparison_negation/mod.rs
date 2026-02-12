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
        expect_no_lint("!(x == y | y == z)", "comparison_negation", None);
        expect_no_lint("!(x & y)", "comparison_negation", None);
        expect_no_lint("!any(x > y)", "comparison_negation", None);
        expect_no_lint("!!target == 1 ~ 'target'", "comparison_negation", None);
        expect_no_lint("!passes.test[stage == 1]", "comparison_negation", None);

        // TODO: for now, I only catch `!(...)`. This is to stay on the safe
        // side regarding operator precedence, but eventually this could be
        // relaxed to report this case (that lintr reports):
        expect_no_lint("!length(x) > 0", "comparison_negation", None);
    }

    #[test]
    fn test_lint_comparison_negation() {
        assert_snapshot!(
            snapshot_lint("!(x >= y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x >= y)
          | --------- `!(x >= y)` can be simplified.
          |
          = help: Use `x < y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x > y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x > y)
          | -------- `!(x > y)` can be simplified.
          |
          = help: Use `x <= y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x <= y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x <= y)
          | --------- `!(x <= y)` can be simplified.
          |
          = help: Use `x > y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x < y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x < y)
          | -------- `!(x < y)` can be simplified.
          |
          = help: Use `x >= y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x == y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x == y)
          | --------- `!(x == y)` can be simplified.
          |
          = help: Use `x != y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x != y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x != y)
          | --------- `!(x != y)` can be simplified.
          |
          = help: Use `x == y` instead.
        Found 1 error.
        "
        );

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
                    // More involved
                    "!(foo(x + 1) != foo(bar(if (x == 1) 2 else 3)))",
                    "if (TRUE && !(bar > foo)) 1"
                ],
                "comparison_negation",
                None
            )
        );
    }

    #[test]
    fn test_comparison_negation_with_comments_no_fix() {
        assert_snapshot!(
            snapshot_lint("# leading comment\n!(x >= y)"),
            @r"
        warning: comparison_negation
         --> <test>:2:1
          |
        2 | !(x >= y)
          | --------- `!(x >= y)` can be simplified.
          |
          = help: Use `x < y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x \n # hello there \n >= y)"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | / !(x 
        2 | |  # hello there 
        3 | |  >= y)
          | |______- `!(x >= y)` can be simplified.
          |
          = help: Use `x < y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x >= y) # trailing comment"),
            @r"
        warning: comparison_negation
         --> <test>:1:1
          |
        1 | !(x >= y) # trailing comment
          | --------- `!(x >= y)` can be simplified.
          |
          = help: Use `x < y` instead.
        Found 1 error.
        "
        );

        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\n!(x >= y)",
                    "!(x \n # hello there \n >= y)",
                    "!(x >= y) # trailing comment",
                ],
                "comparison_negation",
                None
            )
        );
    }
}
