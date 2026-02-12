pub(crate) mod for_loop_dup_index;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "for_loop_dup_index", None)
    }

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
        // Same index in nested loops
        assert_snapshot!(snapshot_lint("for (x in 1:3) {
                for (x in 1:4) {
                    x
                }
            }"), @r"
        warning: for_loop_dup_index
         --> <test>:2:22
          |
        2 |                 for (x in 1:4) {
          |                      -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Whitespace is ignored
        assert_snapshot!(snapshot_lint("for (x in 1:3) {
                for (  x    in 1:4) {
                    x
                }
            }"), @r"
        warning: for_loop_dup_index
         --> <test>:2:24
          |
        2 |                 for (  x    in 1:4) {
          |                        ----------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Code between outer and inner loop
        assert_snapshot!(snapshot_lint("for (i in 1:3) {
                x <- i * 2
                print(x)
                for (i in 1:4) {
                    i
                }
            }"), @r"
        warning: for_loop_dup_index
         --> <test>:4:22
          |
        4 |                 for (i in 1:4) {
          |                      -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Deeply nested with duplicate at innermost level
        assert_snapshot!(snapshot_lint("for (i in 1:3) {
                for (j in 1:4) {
                    for (i in 1:5) {
                        i
                    }
                }
            }"), @r"
        warning: for_loop_dup_index
         --> <test>:3:26
          |
        3 |                     for (i in 1:5) {
          |                          -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Duplicate at middle level
        assert_snapshot!(snapshot_lint("for (i in 1:3) {
                for (i in 1:4) {
                    for (k in 1:5) {
                        k
                    }
                }
            }"), @r"
        warning: for_loop_dup_index
         --> <test>:2:22
          |
        2 |                 for (i in 1:4) {
          |                      -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
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
