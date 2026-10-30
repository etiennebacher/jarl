pub(crate) mod system_file;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "system_file", None)
    }

    #[test]
    fn test_no_lint_system_file() {
        expect_no_lint("system.file('a', 'b', 'c')", "system_file", None);
        expect_no_lint("system.file(paste('a', 'b', 'c'))", "system_file", None);
        expect_no_lint("system.file(file.path())", "system_file", None);
        expect_no_lint("system.file(file.path(,))", "system_file", None);
        expect_no_lint(
            "system.file(file.path('a', fsep = '-'), package = 'foo')",
            "system_file",
            None,
        );
        expect_no_lint(
            "system.file(file.path(package = 'a'), package = 'foo')",
            "system_file",
            None,
        );
        expect_no_lint("file.path('a', 'b', 'c')", "system_file", None);
        expect_no_lint("pkg::system.file(file.path('a'))", "system_file", None);
        expect_no_lint(
            "system.file(package =, file.path('R'))",
            "system_file",
            None,
        );
    }

    #[test]
    fn test_lint_system_file() {
        assert_snapshot!(
            snapshot_lint("system.file(file.path('path', 'to', 'data'), package = 'foo')"),
            @"
        warning: system_file
         --> <test>:1:1
          |
        1 | system.file(file.path('path', 'to', 'data'), package = 'foo')
          | ------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` directly.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::system.file(file.path('path', 'to', 'data'), package = 'foo')"),
            @"
        warning: system_file
         --> <test>:1:1
          |
        1 | base::system.file(file.path('path', 'to', 'data'), package = 'foo')
          | ------------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` directly.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("system.file('prefix', file.path('path', 'to', 'data'), package = 'foo')"),
            @"
        warning: system_file
         --> <test>:1:1
          |
        1 | system.file('prefix', file.path('path', 'to', 'data'), package = 'foo')
          | ----------------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` directly.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("system.file(base::file.path('path', 'to', 'data'), package = 'foo')"),
            @"
        warning: system_file
         --> <test>:1:1
          |
        1 | system.file(base::file.path('path', 'to', 'data'), package = 'foo')
          | ------------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` directly.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("system.file(file.path('R'), file.path('package', fsep = '-'))"),
            @"
        warning: system_file
         --> <test>:1:1
          |
        1 | system.file(file.path('R'), file.path('package', fsep = '-'))
          | ------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` directly.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("system.file(\n # a comment\nfile.path('path', 'to', 'data'), package = 'foo')"),
            @"
        warning: system_file
         --> <test>:1:1
          |
        1 | / system.file(
        2 | |  # a comment
        3 | | file.path('path', 'to', 'data'), package = 'foo')
          | |_________________________________________________- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` directly.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_system_file_fixes() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "system.file(file.path('path', 'to', 'data'), package = 'foo')",
                    "system.file(file.path('path', 'to', 'data'))",
                    "base::system.file(file.path('path', 'to', 'data'), package = 'foo')",
                    "system.file('prefix', file.path('path', 'to', 'data'), package = 'foo')",
                    "system.file(base::file.path('path', 'to', 'data'), package = 'foo')",
                    "system.file(file.path('path'), 'prefix', base::file.path('to', 'data'), package = 'foo')",
                    "system.file(file.path('path'), file.path('to', 'data'), package = 'foo')",
                    "system.file(file.path('R'), file.path('package', fsep = '-'))",
                    "system.file(file.path('package', fsep = '-'), base::file.path('R'))",
                    "system.file(file.path(), file.path('R'))",
                    "system.file(\n # leading comment\nfile.path('path', 'to', 'data'), package = 'foo')",
                    "system.file(file.path(\n # inner comment\n'path', 'to', 'data'), package = 'foo')",
                    "system.file(file.path('path', 'to', 'data'),\n # between arguments\npackage = 'foo')",
                ],
                "system_file",
                None
            )
        );
    }
}
