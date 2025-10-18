pub(crate) mod all_equal;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_all_equal() {
        expect_no_lint("all.equal(a, b)", "all_equal", None);
        expect_no_lint("all.equal(a, b, tolerance = 1e-3)", "all_equal", None);
        expect_no_lint("if (isFALSE(x)) 1", "all_equal", None);
        expect_no_lint(
            "if (isTRUE(all.equal(a, b))) message('equal')",
            "all_equal",
            None,
        );
        expect_no_lint(
            "if (!isTRUE(all.equal(a, b))) message('different')",
            "all_equal",
            None,
        );
        expect_no_lint("if (A) all.equal(x, y)", "all_equal", None);
    }

    #[test]
    fn test_lint_all_equal() {
        use insta::assert_snapshot;

        let expected_message = "Wrap `all.equal()` in `isTRUE()`";
        let expected_message_2 = "Use `!isTRUE()` to check for differences";
        expect_lint(
            "if (all.equal(a, b, tolerance = 1e-3)) message('equal')",
            expected_message,
            "all_equal",
            None,
        );
        expect_lint(
            "if (all.equal(a, b)) message('equal')",
            expected_message,
            "all_equal",
            None,
        );
        expect_lint("!all.equal(a, b)", expected_message, "all_equal", None);
        expect_lint(
            "while (all.equal(a, b)) message('equal')",
            expected_message,
            "all_equal",
            None,
        );
        expect_lint(
            "isFALSE(all.equal(a, b))",
            expected_message_2,
            "all_equal",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "if (all.equal(a, b, tolerance = 1e-3)) message('equal')",
                    "if (all.equal(a, b)) message('equal')",
                    "!all.equal(a, b)",
                    "while (all.equal(a, b)) message('equal')",
                    "isFALSE(all.equal(a, b))",
                    "if (
  # A comment
  all.equal(a, b)
) message('equal')",
                ],
                "all_equal",
            )
        );
    }

    #[test]
    fn test_all_equal_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nif (all.equal(a, b)) message('equal')",
                    "if (all.equal(a,\n# a comment\n b)) message('equal')",
                    "if (all.equal(a, b)) message('equal') # trailing comment",
                ],
                "all_equal",
            )
        );
    }
}
