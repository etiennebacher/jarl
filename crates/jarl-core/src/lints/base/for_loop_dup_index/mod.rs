pub(crate) mod for_loop_dup_index;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_for_loop_dup_index() {
        // Different index variables in nested loops
        expect_no_lint(
            "for (x_outer in 1:3) {
                for (x_inner in 1:4) {
                    x_inner
                }
            }",
            "for_loop_dup_index",
            None,
        );

        // Sequential (non-nested) loops with the same index are fine
        expect_no_lint(
            "{
                for (i in 1:3) {
                    i
                }
                for (i in 1:4) {
                    i
                }
            }",
            "for_loop_dup_index",
            None,
        );

        // Deeply nested loops with unique indices
        expect_no_lint(
            "for (i in 1:3) {
                for (j in 1:4) {
                    for (k in 1:5) {
                        i + j + k
                    }
                }
            }",
            "for_loop_dup_index",
            None,
        );
    }

    #[test]
    fn test_lint_for_loop_dup_index() {
        use insta::assert_snapshot;

        let expected_message = "This index variable is already used in a parent `for` loop.";

        // Same index in nested loops
        expect_lint(
            "for (x in 1:3) {
                for (x in 1:4) {
                    x
                }
            }",
            expected_message,
            "for_loop_dup_index",
            None,
        );

        // Code between outer and inner loop
        expect_lint(
            "for (i in 1:3) {
                x <- i * 2
                print(x)
                for (i in 1:4) {
                    i
                }
            }",
            expected_message,
            "for_loop_dup_index",
            None,
        );

        // Deeply nested with duplicate at innermost level
        expect_lint(
            "for (i in 1:3) {
                for (j in 1:4) {
                    for (i in 1:5) {
                        i
                    }
                }
            }",
            expected_message,
            "for_loop_dup_index",
            None,
        );

        // Duplicate at middle level
        expect_lint(
            "for (i in 1:3) {
                for (i in 1:4) {
                    for (k in 1:5) {
                        k
                    }
                }
            }",
            expected_message,
            "for_loop_dup_index",
            None,
        );

        // With code between loops
        expect_diagnostic_highlight(
            "for (i in 1:3) {
                x <- i * 2
                for (i in 1:4) {
                    i
                }
            }",
            "for_loop_dup_index",
            "i in 1:4",
        );

        // No fixes
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "for (x in 1:3) {
    for (x in 1:4) {
        x
    }
}"
                ],
                "for_loop_dup_index",
                None
            )
        );
    }

    #[test]
    fn test_for_loop_dup_index_diagnostic_ranges() {
        use crate::utils_test::expect_diagnostic_highlight;

        // Diagnostic highlights the inner loop's `index in sequence`
        expect_diagnostic_highlight(
            "for (x in 1:3) {
                for (x in 1:4) {
                    x
                }
            }",
            "for_loop_dup_index",
            "x in 1:4",
        );
    }
}
