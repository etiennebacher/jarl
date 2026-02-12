pub(crate) mod seq;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "seq", None)
    }

    #[test]
    fn test_no_lint_seq() {
        expect_no_lint("1:10", "seq", None);
        expect_no_lint("2:length(x)", "seq", None);
        expect_no_lint("1:(length(x) || 1)", "seq", None);
        expect_no_lint("1:foo(x)", "seq", None);

        // TODO: would be nice to support that
        expect_no_lint("1:dim(x)[1]", "seq", None);
        expect_no_lint("1:dim(x)[[1]]", "seq", None);
    }

    #[test]
    fn test_lint_seq() {
        assert_snapshot!(
            snapshot_lint("1:length(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1:length(x)
          | ----------- `1:length(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1:nrow(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1:nrow(x)
          | --------- `1:nrow(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(nrow((...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1:ncol(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1:ncol(x)
          | --------- `1:ncol(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(ncol(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1:NROW(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1:NROW(x)
          | --------- `1:NROW(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(NROW(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1:NCOL(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1:NCOL(x)
          | --------- `1:NCOL(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(NCOL(...))` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(

            snapshot_lint("1:base::length(x)"),

            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1:base::length(x)
          | ----------------- `1:length(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );

        // Same with 1L
        assert_snapshot!(
            snapshot_lint("1L:length(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1L:length(x)
          | ------------ `1:length(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1L:nrow(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1L:nrow(x)
          | ---------- `1:nrow(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(nrow((...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1L:ncol(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1L:ncol(x)
          | ---------- `1:ncol(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(ncol(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1L:NROW(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1L:NROW(x)
          | ---------- `1:NROW(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(NROW(...))` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1L:NCOL(x)"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | 1L:NCOL(x)
          | ---------- `1:NCOL(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_len(NCOL(...))` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "1:length(x)",
                    "1:nrow(x)",
                    "1:ncol(x)",
                    "1:NROW(x)",
                    "1:NCOL(x)",
                    // Same with 1L
                    "1L:length(x)",
                    "1L:nrow(x)",
                    "1L:ncol(x)",
                    "1L:NROW(x)",
                    "1L:NCOL(x)",
                    "1:length(foo(x))"
                ],
                "seq",
                None
            )
        );
    }

    #[test]
    fn test_seq_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("1:length(\n # a comment \nfoo(x))"),
            @r"
        warning: seq
         --> <test>:1:1
          |
        1 | / 1:length(
        2 | |  # a comment 
        3 | | foo(x))
          | |_______- `1:length(...)` can be wrong if the RHS is 0.
          |
          = help: Use `seq_along(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec!["1:length(\n # a comment \nfoo(x))",],
                "any_is_na",
                None
            )
        );
    }
}
