pub(crate) mod sort;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "sort", None)
    }

    #[test]
    fn test_no_lint_sort() {
        expect_no_lint("x[]", "sort", None);
        expect_no_lint("x[,]", "sort", None);
        expect_no_lint("x[,order(x)]", "sort", None);
        expect_no_lint("x[order(x),]", "sort", None);
        expect_no_lint("x[order(x), 'foo']", "sort", None);
        expect_no_lint("order(x)", "sort", None);
        expect_no_lint("x[order()]", "sort", None);
        expect_no_lint("x[order(y)]", "sort", None);
        expect_no_lint("x[order(x, y)]", "sort", None);
        expect_no_lint("x[c(order(x))]", "sort", None);
    }

    #[test]
    fn test_lint_sort() {
        assert_snapshot!(
            snapshot_lint("x[order(x)]"),
            @r"
        warning: sort
         --> <test>:1:1
          |
        1 | x[order(x)]
          | ----------- `x[order(x)]` is inefficient.
          |
          = help: Use `sort(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[order(x, decreasing = TRUE)]"),
            @r"
        warning: sort
         --> <test>:1:1
          |
        1 | x[order(x, decreasing = TRUE)]
          | ------------------------------ `x[order(x)]` is inefficient.
          |
          = help: Use `sort(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[order(x, na.last = TRUE)]"),
            @r"
        warning: sort
         --> <test>:1:1
          |
        1 | x[order(x, na.last = TRUE)]
          | --------------------------- `x[order(x)]` is inefficient.
          |
          = help: Use `sort(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[order(x, method = \"radix\")]"),
            @r#"
        warning: sort
         --> <test>:1:1
          |
        1 | x[order(x, method = "radix")]
          | ----------------------------- `x[order(x)]` is inefficient.
          |
          = help: Use `sort(x)` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("x[order(x, method = \"radix\", na.last = TRUE)]"),
            @r#"
        warning: sort
         --> <test>:1:1
          |
        1 | x[order(x, method = "radix", na.last = TRUE)]
          | --------------------------------------------- `x[order(x)]` is inefficient.
          |
          = help: Use `sort(x)` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("x[order(method = \"radix\", na.last = TRUE, x)]"),
            @r#"
        warning: sort
         --> <test>:1:1
          |
        1 | x[order(method = "radix", na.last = TRUE, x)]
          | --------------------------------------------- `x[order(x)]` is inefficient.
          |
          = help: Use `sort(x)` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x[order(x)]",
                    "x[order(x, decreasing = TRUE)]",
                    "x[order(x, na.last = TRUE)]",
                    "x[order(x, method = \"radix\")]",
                    "x[order(x, method = \"radix\", na.last = TRUE)]",
                    "x[order(method = \"radix\", na.last = TRUE, x)]",
                ],
                "sort",
                None
            )
        );
    }

    #[test]
    fn test_sort_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx[order(x)]",
                    "x[\n  # comment\n  order(x)\n]",
                    "x[order(\n    # comment\n    x\n  )]",
                    "x[order(x)] # trailing comment",
                ],
                "sort",
                None
            )
        );
    }
}
