pub(crate) mod vector_logic;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "vector_logic", None)
    }

    #[test]
    fn test_no_lint_vector_logic() {
        expect_no_lint("if (x && y) 1", "vector_logic", None);
        expect_no_lint("while (x && y) 1", "vector_logic", None);
        expect_no_lint("if (agg_function(x & y)) 1", "vector_logic", None);
        expect_no_lint("if (DT[x | y, cond]) 1", "vector_logic", None);
        expect_no_lint("if (TRUE && any(TRUE | FALSE)) 1", "vector_logic", None);

        // Bitwise operations with raw/octmode/hexmode
        expect_no_lint("if (info & as.raw(12)) { }", "vector_logic", None);
        expect_no_lint("if (as.raw(12) & info) { }", "vector_logic", None);
        expect_no_lint("if (info | as.raw(12)) { }", "vector_logic", None);
        expect_no_lint("if (info & as.octmode('100')) { }", "vector_logic", None);
        expect_no_lint("if (info | as.octmode('011')) { }", "vector_logic", None);
        expect_no_lint("if (info & as.hexmode('100')) { }", "vector_logic", None);
        expect_no_lint("if (info | as.hexmode('011')) { }", "vector_logic", None);

        // Implicit as.octmode() coercion with strings
        expect_no_lint("if (info & '100') { }", "vector_logic", None);
        expect_no_lint("if (info | '011') { }", "vector_logic", None);
        expect_no_lint("if ('011' | info) { }", "vector_logic", None);
    }

    #[test]
    fn test_lint_vector_logic() {
        assert_snapshot!(
            snapshot_lint("if (TRUE & FALSE) 1"),
            @r"
        warning: vector_logic
         --> <test>:1:5
          |
        1 | if (TRUE & FALSE) 1
          |     ------------ `&` in `if()` statements can be inefficient.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (TRUE | FALSE) 1"),
            @r"
        warning: vector_logic
         --> <test>:1:5
          |
        1 | if (TRUE | FALSE) 1
          |     ------------ `|` in `if()` statements can be inefficient.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (TRUE | FALSE & TRUE) 1"),
            @r"
        warning: vector_logic
         --> <test>:1:5
          |
        1 | if (TRUE | FALSE & TRUE) 1
          |     ------------------- `|` in `if()` statements can be inefficient.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (TRUE & FALSE) 1"),
            @r"
        warning: vector_logic
         --> <test>:1:8
          |
        1 | while (TRUE & FALSE) 1
          |        ------------ `&` in `while()` statements can be inefficient.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (TRUE | FALSE) 1"),
            @r"
        warning: vector_logic
         --> <test>:1:8
          |
        1 | while (TRUE | FALSE) 1
          |        ------------ `|` in `while()` statements can be inefficient.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if ((x > 1) & (y < 2)) 1"),
            @r"
        warning: vector_logic
         --> <test>:1:5
          |
        1 | if ((x > 1) & (y < 2)) 1
          |     ----------------- `&` in `if()` statements can be inefficient.
          |
        Found 1 error.
        "
        );

        // No fixes because `&` and `|` can be S3 methods.
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(vec!["if (x & y) 1",], "class_equals", None)
        );
    }
}
