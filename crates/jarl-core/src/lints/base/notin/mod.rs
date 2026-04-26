pub(crate) mod notin;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "notin", Some("4.6"))
    }

    #[test]
    fn test_lint_notin() {
        assert_snapshot!(
            snapshot_lint("!(x %in% y)"),
            @"
        warning: notin
         --> <test>:1:1
          |
        1 | !(x %in% y)
          | ----------- `!(x %in% y)` can be simplified.
          |
          = help: Use `x %notin% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!x %in% y"),
            @"
        warning: notin
         --> <test>:1:1
          |
        1 | !x %in% y
          | --------- `!x %in% y` can be simplified.
          |
          = help: Use `x %notin% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(
                r#"if (!(1 %in% c(1, 2, 3))) {
  print("1 is not in the vector")
}"#
            ),
            @r#"
        warning: notin
         --> <test>:1:5
          |
        1 | if (!(1 %in% c(1, 2, 3))) {
          |     -------------------- `!(x %in% y)` can be simplified.
          |
          = help: Use `x %notin% y` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "!(x %in% y)",
                    "!x %in% y",
                    "if (!(1 %in% c(1, 2, 3))) print('not in vector')",
                    "if (!1 %in% c(1, 2, 3)) print('not in vector')",
                    "!(foo(x + 1) %in% bar(y + 1))",
                    "!foo(x + 1) %in% bar(y + 1)",
                ],
                "notin",
                Some("4.6")
            )
        );
    }

    #[test]
    fn test_no_lint_notin() {
        expect_no_lint("x %in% y", "notin", Some("4.6"));
        expect_no_lint("!(x == y)", "notin", Some("4.6"));
        expect_no_lint("-(x %in% y)", "notin", Some("4.6"));
        expect_no_lint("!((x %in% y))", "notin", Some("4.6"));
        expect_no_lint("!(NA %in% x)", "notin", Some("4.6"));
        expect_no_lint("!NA %in% x", "notin", Some("4.6"));
        expect_no_lint("!(x %in% NA)", "notin", Some("4.6"));
        expect_no_lint("!x %in% NA", "notin", Some("4.6"));
        expect_no_lint("!(x %in% NA_character_)", "notin", Some("4.6"));
        expect_no_lint("!(x %in% y)", "notin", Some("4.5"));
        expect_no_lint("!x %in% y", "notin", Some("4.5"));
        expect_no_lint("!(x %in% y)", "notin", None);
        expect_no_lint("!x %in% y", "notin", None);
    }

    #[test]
    fn test_notin_with_comments_no_fix() {
        assert_snapshot!(
            snapshot_lint("# leading comment\n!(x %in% y)"),
            @"
        warning: notin
         --> <test>:2:1
          |
        2 | !(x %in% y)
          | ----------- `!(x %in% y)` can be simplified.
          |
          = help: Use `x %notin% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("!(x \n # comment\n %in% y)"),
            @"
        warning: notin
         --> <test>:1:1
          |
        1 | / !(x 
        2 | |  # comment
        3 | |  %in% y)
          | |________- `!(x %in% y)` can be simplified.
          |
          = help: Use `x %notin% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec!["!(x \n # comment\n %in% y)", "!x %in%\n # comment\n y"],
                "notin",
                Some("4.6")
            )
        );
    }
}
