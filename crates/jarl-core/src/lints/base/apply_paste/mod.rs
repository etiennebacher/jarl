pub(crate) mod apply_paste;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "apply_paste", None)
    }

    #[test]
    fn test_no_lint_apply_paste() {
        // Not `paste`
        expect_no_lint("apply(x, 1, sum, collapse = \"_\")", "apply_paste", None);
        // No `collapse`
        expect_no_lint("apply(x, 1, paste)", "apply_paste", None);
        // Margin is not 1
        expect_no_lint("apply(x, 2, paste, collapse = \"_\")", "apply_paste", None);
        // Extra argument forwarded to `paste` that we can't translate
        expect_no_lint(
            "apply(x, 1, paste, sep = \" \", collapse = \"_\")",
            "apply_paste",
            None,
        );
        // Not `apply`
        expect_no_lint("sapply(x, 1, paste, collapse = \"_\")", "apply_paste", None);
        // Do not panic (no value for `X`)
        expect_no_lint("apply(X=, 1, paste, collapse = \"_\")", "apply_paste", None);
    }

    #[test]
    fn test_lint_apply_paste() {
        assert_snapshot!(
            snapshot_lint("apply(test[, c(\"x\", \"y\")], 1, paste, collapse = \"_\")"),
            @r#"
        warning: apply_paste
         --> <test>:1:1
          |
        1 | apply(test[, c("x", "y")], 1, paste, collapse = "_")
          | ---------------------------------------------------- `apply()` with `paste()` is inefficient.
          |
          = help: Use `do.call(paste, c(x, sep = ...))` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("apply(x, 1L, paste, collapse = \"_\")"),
            @r#"
        warning: apply_paste
         --> <test>:1:1
          |
        1 | apply(x, 1L, paste, collapse = "_")
          | ----------------------------------- `apply()` with `paste()` is inefficient.
          |
          = help: Use `do.call(paste, c(x, sep = ...))` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("apply(x, MARGIN = 1, FUN = paste, collapse = \"_\")"),
            @r#"
        warning: apply_paste
         --> <test>:1:1
          |
        1 | apply(x, MARGIN = 1, FUN = paste, collapse = "_")
          | ------------------------------------------------- `apply()` with `paste()` is inefficient.
          |
          = help: Use `do.call(paste, c(x, sep = ...))` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("base::apply(x, 1, base::paste, collapse = \"_\")"),
            @r#"
        warning: apply_paste
         --> <test>:1:1
          |
        1 | base::apply(x, 1, base::paste, collapse = "_")
          | ---------------------------------------------- `apply()` with `paste()` is inefficient.
          |
          = help: Use `do.call(paste, c(x, sep = ...))` instead.
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_apply_paste_fix() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "apply(test[, c(\"x\", \"y\")], 1, paste, collapse = \"_\")",
                    "apply(x, 1L, paste, collapse = \"_\")",
                    "apply(x, MARGIN = 1, FUN = paste, collapse = \"_\")",
                    "apply(MARGIN = 1, FUN = paste, X = x, collapse = \"_\")",
                ],
                "apply_paste",
            )
        );
    }

    #[test]
    fn test_apply_paste_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\napply(x, 1, paste, collapse = \"_\")",
                    "apply(x, 1, paste, collapse = \"_\") # trailing comment",
                ],
                "apply_paste",
            )
        );
    }
}
