pub(crate) mod redundant_ifelse;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "redundant_ifelse", None)
    }

    #[test]
    fn test_no_lint_redundant_ifelse() {
        // Normal ifelse calls with non-boolean constants
        expect_no_lint("ifelse(x > 0, 1, 0)", "redundant_ifelse", None);
        expect_no_lint("ifelse(x > 0, 'yes', 'no')", "redundant_ifelse", None);
        expect_no_lint("ifelse(x > 0, x, y)", "redundant_ifelse", None);
        expect_no_lint("ifelse(x > 0, TRUE, 0)", "redundant_ifelse", None);
        expect_no_lint("ifelse(x > 0, 1, FALSE)", "redundant_ifelse", None);

        // if_else with non-boolean constants
        expect_no_lint("dplyr::if_else(x > 0, 1, 0)", "redundant_ifelse", None);
        expect_no_lint("if_else(x > 0, 'yes', 'no')", "redundant_ifelse", None);

        // fifelse with non-boolean constants
        expect_no_lint("data.table::fifelse(x > 0, 1, 0)", "redundant_ifelse", None);
        expect_no_lint("fifelse(x > 0, x, y)", "redundant_ifelse", None);

        // Calls with more than 3 arguments (shouldn't be handled)
        expect_no_lint("ifelse(x > 0, TRUE, FALSE, NA)", "redundant_ifelse", None);
        expect_no_lint(
            "dplyr::if_else(x > 0, TRUE, FALSE, missing = NA)",
            "redundant_ifelse",
            None,
        );

        // Other functions that aren't ifelse
        expect_no_lint("if (x > 0) TRUE else FALSE", "redundant_ifelse", None);
        expect_no_lint("my_ifelse(x > 0, TRUE, FALSE)", "redundant_ifelse", None);
    }

    #[test]
    fn test_redundant_ifelse_complex_conditions() {
        // Complex conditions should still be detected
        assert_snapshot!(
            snapshot_lint("ifelse(x > 0 & y < 10, TRUE, FALSE)"),
            @r"
        warning: redundant_ifelse
         --> <test>:1:1
          |
        1 | ifelse(x > 0 & y < 10, TRUE, FALSE)
          | ----------------------------------- This `ifelse()` is redundant.
          |
          = help: Use `condition` directly.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("ifelse(foo(bar(x)) == 'test', TRUE, FALSE)"),
            @r"
        warning: redundant_ifelse
         --> <test>:1:1
          |
        1 | ifelse(foo(bar(x)) == 'test', TRUE, FALSE)
          | ------------------------------------------ This `ifelse()` is redundant.
          |
          = help: Use `condition` directly.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "complex_conditions",
            get_fixed_text(
                vec![
                    "ifelse(x > 0 & y < 10, TRUE, FALSE)",
                    "ifelse(x > 0 | y < 10, FALSE, TRUE)",
                    "ifelse(foo(bar(x)) == 'test', TRUE, FALSE)",
                    "ifelse(!is.na(x) & x > 0, TRUE, FALSE)",
                ],
                "redundant_ifelse",
                None
            )
        );
    }

    #[test]
    fn test_redundant_ifelse_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nifelse(x > 0, TRUE, FALSE)",
                    "ifelse(\n  # comment\n  x > 0, TRUE, FALSE)",
                    "ifelse(x > 0, TRUE, FALSE) # trailing comment",
                ],
                "redundant_ifelse",
                None
            )
        );
    }

    #[test]
    fn test_redundant_ifelse_all_variants() {
        // Comprehensive test with all function variants and patterns
        assert_snapshot!(
            "all_variants",
            get_fixed_text(
                vec![
                    // ifelse variants
                    "ifelse(x > 0, TRUE, FALSE)",
                    "ifelse(x > 0, FALSE, TRUE)",
                    "ifelse(x > 0, TRUE, TRUE)",
                    "ifelse(x > 0, FALSE, FALSE)",
                    // if_else variants
                    "if_else(x > 0, TRUE, FALSE)",
                    "if_else(x > 0, FALSE, TRUE)",
                    "if_else(x > 0, TRUE, TRUE)",
                    "if_else(x > 0, FALSE, FALSE)",
                    // fifelse variants
                    "fifelse(x > 0, TRUE, FALSE)",
                    "fifelse(x > 0, FALSE, TRUE)",
                    "fifelse(x > 0, TRUE, TRUE)",
                    "fifelse(x > 0, FALSE, FALSE)",
                    // With namespace
                    "base::ifelse(x > 0, TRUE, FALSE)",
                    "dplyr::if_else(x > 0, TRUE, FALSE)",
                    "data.table::fifelse(x > 0, TRUE, FALSE)",
                ],
                "redundant_ifelse",
                None
            )
        );
    }
}
