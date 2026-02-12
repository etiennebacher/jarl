pub(crate) mod sprintf;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "sprintf", None)
    }

    #[test]
    fn test_no_lint_sprintf() {
        expect_no_lint("sprintf(1)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', 1)", "sprintf", None);
        expect_no_lint("sprintf(fmt = 'hello %d', 1)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', x)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', x + 1)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', f(x))", "sprintf", None);
        expect_no_lint("sprintf('hello %1$s %1$s', x)", "sprintf", None);
        expect_no_lint("sprintf('hello %2$d %1$s %1$s', x, y)", "sprintf", None);
        expect_no_lint("sprintf('%05.1f', pi)", "sprintf", None);

        // Allow multi-digit index
        expect_no_lint(
            "sprintf('hello %1$s %2$s %3$s %4$s %5$s %6$s %7$s %8$s %9$s %10$s', x1, x2, x3, x4, x5, x6, x7, x8, x9, x10)",
            "sprintf",
            None,
        );
        // Mix of indexed and non-indexed special chars
        expect_no_lint("sprintf('hello %1$s %s', '1')", "sprintf", None);
        // Whitespace between "%" and special char is allowed.
        expect_no_lint("sprintf('%   s', 1)", "sprintf", None);

        // Found in lrberge/stringmagic
        expect_no_lint(
            "sprintf(\"%s%.*s\", \"abc\", 1, \"0000000000000000\")",
            "sprintf",
            None,
        );
        expect_no_lint("sprintf(\"% *s\", 3, \"  \")", "sprintf", None);

        // Don't know how to handle pipes for now
        expect_no_lint("'abc' |> sprintf('%s', x = _)", "sprintf", None);
    }

    #[test]
    fn test_lint_sprintf_no_arg() {
        assert_snapshot!(
            snapshot_lint("sprintf('a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('a')
          | ------------ `sprintf()` without special characters is useless.
          |
          = help: Use directly the input of `sprintf()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sprintf(\"a\")"),
            @r#"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf("a")
          | ------------ `sprintf()` without special characters is useless.
          |
          = help: Use directly the input of `sprintf()` instead.
        Found 1 error.
        "#
        );
        // "%%" is used to escape the "%" symbol
        assert_snapshot!(
            snapshot_lint("sprintf('%%')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%%')
          | ------------- `sprintf()` without special characters is useless.
          |
          = help: Use directly the input of `sprintf()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sprintf('%%', '')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%%', '')
          | ----------------- `sprintf()` without special characters is useless.
          |
          = help: Use directly the input of `sprintf()` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "sprintf('a')",
                    "sprintf(\"a\")",
                    "sprintf('%%')",
                    "sprintf('hello %%')",
                ],
                "sprintf",
                None
            )
        );
    }

    #[test]
    fn test_lint_sprintf_mismatch() {
        assert_snapshot!(
            snapshot_lint("sprintf('%a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%a')
          | ------------- Mismatch between number of special characters and number of arguments.
          |
          = help: Found 1 special character(s) and 0 argument(s).
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sprintf('%a %s', 1)"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%a %s', 1)
          | ------------------- Mismatch between number of special characters and number of arguments.
          |
          = help: Found 2 special character(s) and 1 argument(s).
        Found 1 error.
        "
        );
        // Mix of indexed and non-indexed special chars
        assert_snapshot!(
            snapshot_lint("sprintf('hello %1$s %s', '1', '2')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('hello %1$s %s', '1', '2')
          | ---------------------------------- Mismatch between number of special characters and number of arguments.
          |
          = help: Found 1 special character(s) and 2 argument(s).
        Found 1 error.
        "
        );

        // No fixes because this pattern generates an error at runtime. User
        // needs to fix it.
        assert_snapshot!(
            "no_fix_mismatch",
            get_fixed_text(
                vec!["sprintf('%a')", "sprintf('%a %s', 1)",],
                "sprintf",
                None
            )
        );
    }

    #[test]
    fn test_lint_sprintf_wrong_special_chars() {
        assert_snapshot!(
            snapshot_lint("sprintf('%y', 'a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%y', 'a')
          | ------------------ `sprintf()` contains some invalid `%`.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sprintf('%', 'a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%', 'a')
          | ----------------- `sprintf()` contains some invalid `%`.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sprintf('1%', 'a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('1%', 'a')
          | ------------------ `sprintf()` contains some invalid `%`.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sprintf('%s%', 'a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | sprintf('%s%', 'a')
          | ------------------- `sprintf()` contains some invalid `%`.
          |
        Found 1 error.
        "
        );

        // No fixes because this pattern generates an error at runtime. User
        // needs to fix it.
        assert_snapshot!(
            "no_fix_wrong_special_chars",
            get_fixed_text(vec!["sprintf('%y', 'a')",], "sprintf", None)
        );
    }

    #[test]
    fn test_sprintf_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("sprintf(\n # a comment \n'a')"),
            @r"
        warning: sprintf
         --> <test>:1:1
          |
        1 | / sprintf(
        2 | |  # a comment 
        3 | | 'a')
          | |____- `sprintf()` without special characters is useless.
          |
          = help: Use directly the input of `sprintf()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsprintf('a')",
                    "sprintf(\n # a comment \n'a')",
                    "sprintf('a') # trailing comment",
                ],
                "sprintf",
                None
            )
        );
    }
}
