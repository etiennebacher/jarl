pub(crate) mod download_file;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "download_file", None)
    }

    #[test]
    fn test_no_lint_download_file() {
        expect_no_lint("download.file(x, mode = 'ab')", "download_file", None);
        expect_no_lint("download.file(x, mode = 'wb')", "download_file", None);
        expect_no_lint("download.file(x, y, z, w, 'ab')", "download_file", None);
        expect_no_lint("download.file(x, y, z, w, 'wb')", "download_file", None);
        expect_no_lint(
            "download.file(x, y, z, method = 'curl', 'a')",
            "download_file",
            None,
        );
        expect_no_lint(
            "download.file(x, y, z, method = 'curl', 'w')",
            "download_file",
            None,
        );
        expect_no_lint(
            "download.file(x, y, z, method = 'wget', 'a')",
            "download_file",
            None,
        );
        expect_no_lint(
            "download.file(x, y, z, method = 'wget', 'w')",
            "download_file",
            None,
        );
    }

    #[test]
    fn test_lint_download_file() {
        assert_snapshot!(
            snapshot_lint("download.file(x)"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x)
          | ---------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("utils::download.file(x)"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | utils::download.file(x)
          | ----------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, mode = 'a')"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, mode = 'a')
          | ---------------------------- `download.file()` with `mode = 'a'` can cause portability issues on Windows.
          |
          = help: Use mode = 'ab' instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, mode = 'w')"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, mode = 'w')
          | ---------------------------- `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, mode = \"a\")"),
            @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, mode = "a")
          | ---------------------------- `download.file()` with `mode = 'a'` can cause portability issues on Windows.
          |
          = help: Use mode = 'ab' instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, mode = \"w\")"),
            @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, mode = "w")
          | ---------------------------- `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, y, z, w, 'a')"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, z, w, 'a')
          | ------------------------------ `download.file()` with `mode = 'a'` can cause portability issues on Windows.
          |
          = help: Use mode = 'ab' instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, y, z, w, 'w')"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, z, w, 'w')
          | ------------------------------ `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "
        );
        // Only method = "wget" / "curl" don't trigger the lint.
        assert_snapshot!(
            snapshot_lint("download.file(x, y, z, method = 'foo', 'a')"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, z, method = 'foo', 'a')
          | ------------------------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("download.file(x, y, z, method = 'foo', 'w')"),
            @r"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, z, method = 'foo', 'w')
          | ------------------------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "
        );
    }
}
