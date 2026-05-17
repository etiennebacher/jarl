pub(crate) mod glue;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "glue", None)
    }

    #[test]
    fn test_lint_glue() {
        assert_snapshot!(
            snapshot_lint("glue(\"abc\")"),
            @r#"
        warning: glue
         --> <test>:1:1
          |
        1 | glue("abc")
          | ----------- glue() with a constant string performs no interpolation.
          |
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("glue('{a}', .open = '<', .close = '>')"),
            @r"
        warning: glue
         --> <test>:1:1
          |
        1 | glue('{a}', .open = '<', .close = '>')
          | -------------------------------------- Using glue() with .open and .close when the string does not contain the specified delimiters is useless.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_glue() {
        expect_no_lint("glue('<a}', .open = '<')", "glue", None);
        expect_no_lint("glue('{a}', .close = '}')", "glue", None);
        expect_no_lint("glue('{a}', '{b}')", "glue", None);
    }
}
