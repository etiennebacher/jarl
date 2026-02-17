pub(crate) mod any_duplicated;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "any_duplicated", None)
    }

    #[test]
    fn test_no_lint_any_duplicated() {
        expect_no_lint("any(x)", "any_duplicated", None);
        expect_no_lint("duplicated(x)", "any_duplicated", None);
        expect_no_lint("any(!duplicated(x))", "any_duplicated", None);
        expect_no_lint("any(!duplicated(foo(x)))", "any_duplicated", None);
        expect_no_lint("any(na.rm = TRUE)", "any_duplicated", None);
        expect_no_lint("any()", "any_duplicated", None);
        // Incomplete pipe chains should not trigger
        expect_no_lint("x |> any()", "any_duplicated", None);
        expect_no_lint("x |> duplicated()", "any_duplicated", None);
        expect_no_lint(
            "x |> any() |> mean() |> duplicated()",
            "any_duplicated",
            None,
        );
    }

    #[test]
    fn test_lint_any_duplicated() {
        assert_snapshot!(
            snapshot_lint("any(duplicated(x))"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | any(duplicated(x))
          | ------------------ `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("any(duplicated(foo(x)))"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | any(duplicated(foo(x)))
          | ----------------------- `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("any(duplicated(x), na.rm = TRUE)"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | any(duplicated(x), na.rm = TRUE)
          | -------------------------------- `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("any(na.rm = TRUE, duplicated(x))"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | any(na.rm = TRUE, duplicated(x))
          | -------------------------------- `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("any(duplicated(x)); 1 + 1; any(duplicated(y))"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | any(duplicated(x)); 1 + 1; any(duplicated(y))
          | ------------------ `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        warning: any_duplicated
         --> <test>:1:28
          |
        1 | any(duplicated(x)); 1 + 1; any(duplicated(y))
          |                            ------------------ `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 2 errors.
        "
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(duplicated(x))",
                    "any(duplicated(foo(x)))",
                    "any(duplicated(x), na.rm = TRUE)",
                ],
                "any_duplicated",
                None
            )
        );
    }

    #[test]
    fn test_lint_any_duplicated_piped() {
        assert_snapshot!(
            snapshot_lint("duplicated(x) |> \n any()"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | / duplicated(x) |> 
        2 | |  any()
          | |______- `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x |> \n duplicated() |> \n any()"),
            @r"
        warning: any_duplicated
         --> <test>:1:1
          |
        1 | / x |> 
        2 | |  duplicated() |> 
        3 | |  any()
          | |______- `any(duplicated(...))` is inefficient.
          |
          = help: Use `anyDuplicated(...) > 0` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "multiline_pipe",
            get_fixed_text(
                vec![
                    "duplicated(x) |>\n  any()",
                    "x |>\n  duplicated() |>\n  any()",
                ],
                "any_duplicated",
                None
            )
        );
    }

    #[test]
    fn test_any_duplicated_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nany(duplicated(x))",
                    "any(\n  # comment\n  duplicated(x)\n)",
                    "any(duplicated(\n    # comment\n    x\n  ))",
                    "any(duplicated(x)) # trailing comment",
                ],
                "any_duplicated",
                None
            )
        );
    }
}
