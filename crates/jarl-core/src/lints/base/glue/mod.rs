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
          | ----------- This `glue()` call isn't necessary because it performs no interpolation.
          |
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("glue('{a}', .open = '<', .close = '>')"),
            @"
        warning: glue
         --> <test>:1:1
          |
        1 | glue('{a}', .open = '<', .close = '>')
          | -------------------------------------- This `glue()` call isn't necessary because it performs no interpolation.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("glue('a', .sep = ' ')"),
            @"
        warning: glue
         --> <test>:1:1
          |
        1 | glue('a', .sep = ' ')
          | --------------------- This `glue()` call isn't necessary because it performs no interpolation.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("glue(\"{abc\")"),
            @r#"
        warning: glue
         --> <test>:1:1
          |
        1 | glue("{abc")
          | ------------ This `glue()` call contains incomplete delimiters and would error when evaluated.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_no_lint_glue() {
        expect_no_lint("glue('<a}', .open = '<')", "glue", None);
        expect_no_lint("glue('{a}', .close = '}')", "glue", None);
        expect_no_lint("glue('{a}', '{b}')", "glue", None);
    }

    #[test]
    fn test_no_lint_glue_escaped_delimiters() {
        // Doubled delimiters are glue escape sequences and should not trigger the incomplete delimiters lint.
        expect_no_lint(r#"glue("{{x}}")"#, "glue", None);
        expect_no_lint(r#"glue("{x}\n}}")"#, "glue", None);
        expect_no_lint(r#"glue("{x}\n\t\t{{NULL, NULL, 0}}\n}};\n")"#, "glue", None);
    }

    #[test]
    fn test_glue_multibyte_chars() {
        // Multi-byte UTF-8 characters between delimiters must not cause a panic
        // when scanning for incomplete delimiters. This interpolates, so no lint.
        expect_no_lint(r#"glue("{x} – {y}")"#, "glue", None);

        // An incomplete delimiter after a multi-byte char is still reported.
        assert_snapshot!(
            snapshot_lint(r#"glue("{abc – ")"#),
            @r#"
        warning: glue
         --> <test>:1:1
          |
        1 | glue("{abc – ")
          | --------------- This `glue()` call contains incomplete delimiters and would error when evaluated.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_no_lint_glue_from_another_package() {
        expect_no_lint("foo::glue('abc')", "glue", None);
    }

    #[test]
    fn test_no_lint_glue_missing_delimiter() {
        expect_no_lint("glue('x', .close = )", "glue", None);
        expect_no_lint("glue('x', .open = )", "glue", None);
    }

    #[test]
    fn test_glue_raw_string() {
        assert_snapshot!(
            snapshot_lint(r#"glue(r"(abc)")"#),
            @r#"
        warning: glue
         --> <test>:1:1
          |
        1 | glue(r"(abc)")
          | -------------- This `glue()` call isn't necessary because it performs no interpolation.
          |
        Found 1 error.
        "#
        );
        expect_no_lint(r#"glue(r"({name})")"#, "glue", None);
    }
}
