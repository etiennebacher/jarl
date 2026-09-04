pub(crate) mod any_is_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "any_is_na", None)
    }

    #[test]
    fn test_no_lint_any_na() {
        expect_no_lint("any(x)", "any_is_na", None);
        expect_no_lint("any(is.na(x), is.na(y))", "any_is_na", None);
        expect_no_lint("is.na(x)", "any_is_na", None);
        expect_no_lint("any(!is.na(x))", "any_is_na", None);
        expect_no_lint("any(!is.na(foo(x)))", "any_is_na", None);
        expect_no_lint("any()", "any_is_na", None);
        expect_no_lint("any(na.rm = TRUE)", "any_is_na", None);
        expect_no_lint("x %in% NA", "any_is_na", None);
        expect_no_lint("x %notin% NA", "any_is_na", None);
        // Incomplete pipe chains should not trigger
        expect_no_lint("x |> any()", "any_is_na", None);
        expect_no_lint("x |> is.na()", "any_is_na", None);
    }

    #[test]
    fn test_lint_any_na() {
        assert_snapshot!(
            snapshot_lint("any(is.na(x))"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | any(is.na(x))
          | ------------- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("any(is.na(foo(x)))"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | any(is.na(foo(x)))
          | ------------------ `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::any(is.na(foo(x)))"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | base::any(is.na(foo(x)))
          | ------------------------ `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("any(is.na(x), na.rm = TRUE)"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | any(is.na(x), na.rm = TRUE)
          | --------------------------- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("NA %in% x"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | NA %in% x
          | --------- `NA %in% x` is inefficient.
          |
          = help: Use `anyNA(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("NA %notin% x"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | NA %notin% x
          | ------------ `NA %notin% x` is inefficient.
          |
          = help: Use `!anyNA(x)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("is.na(x) |> \n any()"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | / is.na(x) |> 
        2 | |  any()
          | |______- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x |> \n is.na() |> \n any()"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | / x |> 
        2 | |  is.na() |> 
        3 | |  any()
          | |______- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("foo(x) |> \n is.na() |> \n any()"),
            @"
        warning: any_is_na
         --> <test>:1:1
          |
        1 | / foo(x) |> 
        2 | |  is.na() |> 
        3 | |  any()
          | |______- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(is.na(x))",
                    "NA %in% x",
                    "NA %notin% x",
                    "any(is.na(foo(x)))",
                    "any(is.na(x), na.rm = TRUE)",
                ],
                "any_is_na",
                None
            )
        );
    }

    #[test]
    fn test_lint_any_na_multiline_pipe() {
        assert_snapshot!(
            "multiline_pipe",
            get_fixed_text(
                vec![
                    "is.na(x) |>\n  any()",
                    "x |>\n  is.na() |>\n  any()",
                    "foo(x) |>\n  is.na() |>\n  any()",
                ],
                "any_is_na",
                None
            )
        );
    }

    #[test]
    fn test_lint_any_na_in_subset_and_extraction_objects() {
        assert_snapshot!(
            snapshot_lint(
                r#"list(has_na = any(is.na(a)))[1]
list(has_na = any(is.na(b)))[["has_na"]]
list(has_na = any(is.na(c)))$has_na
methods::new("Example", has_na = any(is.na(d)))@has_na"#
            ),
            @r#"
        warning: any_is_na
         --> <test>:1:15
          |
        1 | list(has_na = any(is.na(a)))[1]
          |               ------------- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        warning: any_is_na
         --> <test>:2:15
          |
        2 | list(has_na = any(is.na(b)))[["has_na"]]
          |               ------------- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        warning: any_is_na
         --> <test>:3:15
          |
        3 | list(has_na = any(is.na(c)))$has_na
          |               ------------- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        warning: any_is_na
         --> <test>:4:34
          |
        4 | methods::new("Example", has_na = any(is.na(d)))@has_na
          |                                  ------------- `any(is.na(...))` is inefficient.
          |
          = help: Use `anyNA(...)` instead.
        Found 4 errors.
        "#
        );
    }

    #[test]
    fn test_any_is_na_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nany(is.na(x))",
                    "any(\n  # comment\n  is.na(x)\n)",
                    "any(is.na(\n    # comment\n    x\n  ))",
                    "NA %notin%\n  # comment\n  x",
                    "any(is.na(x)) # trailing comment",
                ],
                "any_is_na",
                None
            )
        );
    }
}
