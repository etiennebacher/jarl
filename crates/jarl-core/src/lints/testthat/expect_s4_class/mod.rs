pub(crate) mod expect_s4_class;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_s4_class", None)
    }

    #[test]
    fn test_no_lint_expect_s4_class() {
        expect_no_lint("expect_s4_class(x, \"Matrix\")", "expect_s4_class", None);
        expect_no_lint("expect_true(x)", "expect_s4_class", None);
        expect_no_lint("expect_true(foo(x, \"Matrix\"))", "expect_s4_class", None);
        expect_no_lint("expect_false(is(x, \"Matrix\"))", "expect_s4_class", None);
        expect_no_lint("expect_true(is(x))", "expect_s4_class", None);
        expect_no_lint("expect_true(is(x, \"A\", \"B\"))", "expect_s4_class", None);
        expect_no_lint(
            "expect_true(is(x, \"Matrix\"), info = \"wrong class\")",
            "expect_s4_class",
            None,
        );
        expect_no_lint(
            "expect_true(is(x, \"Matrix\"), label = \"x\")",
            "expect_s4_class",
            None,
        );

        // Wrong code but no panic.
        expect_no_lint("expect_true()", "expect_s4_class", None);
        expect_no_lint("expect_true(object =)", "expect_s4_class", None);
        expect_no_lint("expect_true(is())", "expect_s4_class", None);
    }

    #[test]
    fn test_lint_expect_s4_class() {
        assert_snapshot!(
            snapshot_lint("expect_true(is(x, \"Matrix\"))"),
            @r#"
        warning: expect_s4_class
         --> <test>:1:1
          |
        1 | expect_true(is(x, "Matrix"))
          | ---------------------------- `expect_s4_class(x, "Matrix")` is better than `expect_true(is(x, "Matrix"))`.
          |
          = help: Use `expect_s4_class(x, "Matrix")` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("expect_true(is(foo(x), class_name))"),
            @r#"
        warning: expect_s4_class
         --> <test>:1:1
          |
        1 | expect_true(is(foo(x), class_name))
          | ----------------------------------- `expect_s4_class(foo(x), class_name)` is better than `expect_true(is(foo(x), class_name))`.
          |
          = help: Use `expect_s4_class(foo(x), class_name)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_true(is(x, \"Matrix\"))",
                    "expect_true(is(foo(x), class_name))",
                    "expect_true(is(class2 = \"Matrix\", object = x))",
                    "testthat::expect_true(methods::is(x, \"Matrix\"))",
                ],
                "expect_s4_class",
                None,
            )
        );
    }

    #[test]
    fn test_expect_s4_class_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_true(is(x, \"Matrix\"))",
                    "expect_true(is(x,\n  # comment\n  \"Matrix\"))",
                    "expect_true(is(x, \"Matrix\")) # trailing comment",
                ],
                "expect_s4_class",
                None,
            )
        );
    }
}
