pub(crate) mod seq2;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "seq2", None)
    }

    #[test]
    fn test_no_lint_seq2() {
        // seq_len(...) or seq_along(...) expressions are fine
        expect_no_lint("seq_len(x)", "seq2", None);
        expect_no_lint("seq_along(x)", "seq2", None);
        expect_no_lint("seq(2, length(x))", "seq2", None);
        expect_no_lint("seq(length(x), 2)", "seq2", None);
        expect_no_lint("seq()", "seq2", None);
        expect_no_lint("seq(foo(x))", "seq2", None);
    }

    #[test]
    fn test_lint_seq2() {
        assert_snapshot!(
            snapshot_lint("seq(length(x))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | seq(length(x))
          | -------------- `seq(length(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::seq(base::length(x))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | base::seq(base::length(x))
          | -------------------------- `seq(length(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("seq(nrow(x))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | seq(nrow(x))
          | ------------ `seq(nrow(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_len(nrow(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("seq(ncol(x))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | seq(ncol(x))
          | ------------ `seq(ncol(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_len(ncol(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("seq(NROW(x))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | seq(NROW(x))
          | ------------ `seq(NROW(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_len(NROW(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("seq(NCOL(x))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | seq(NCOL(x))
          | ------------ `seq(NCOL(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_len(NCOL(...))` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "seq(length(x))",
                    "seq(nrow(x))",
                    "seq(ncol(x))",
                    "seq(NROW(x))",
                    "seq(NCOL(x))",
                    "seq(length(foo(x)))"
                ],
                "seq2",
                None
            )
        );
    }

    #[test]
    fn test_seq2_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("seq(length(\n # a comment \nfoo(x)))"),
            @r"
        warning: seq2
         --> <test>:1:1
          |
        1 | / seq(length(
        2 | |  # a comment 
        3 | | foo(x)))
          | |________- `seq(length(...))` can be wrong if the argument has length 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec!["seq(length(\n # a comment \nfoo(x)))",],
                "any_is_na",
                None
            )
        );
    }
}
