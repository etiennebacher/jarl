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
        for code in [
            "download.file(x, y, mode = 'ab')",
            "download.file(x, y, mode = 'wb')",
            "download.file('https://example.org/data.pdf', y, mode = 'wb')",
            "download.file(x, y, z, FALSE, 'ab')",
            "download.file(x, y, z, FALSE, 'wb')",
            "download.file(x, y, mode = chosen_mode)",
            "download.file(x, y, mode = (('wb')))",
            r#"download.file(x, y, mode = r"(wb)")"#,
            r#"download.file(x, y, mode = "\x77b")"#,
            r#"download.file(x, y, mode = "\141b")"#,
            r#"download.file(x, y, method = "\u0063url")"#,
            r#"download.file(x, y, method = "\U{00000063}url")"#,
            r#"download.file(x, y, mode = r"(\x77)")"#,
            r#"download.file(x, y, mode = "\\x77")"#,
            "download.file(x, y, method = 'curl')",
            "download.file(x, y, method = 'wget')",
            "download.file(x, y, method = (('curl')))",
            r#"download.file(x, y, method = r"(wget)")"#,
            r#"download.file(x, y, method = "\x63url")"#,
            "download.file(x, y, method = 'curl', mode =)",
            "other::download.file(x, y)",
            "other::download.file(x, y, mode = 'w')",
            "other::download.file(x, y, method = 'curl', mode = 'w')",
            "obj$download.file(x, y)",
            "obj$download.file(x, y, mode = 'w')",
            "utils::download.file('https://example.org/data.zip', y)",
            "download.file(destfile = y, url = 'https://example.org/data.zip')",
            "download.file('https://example.org/data.zip', y, mode =)",
            "download.file((('https://example.org/data.zip')), y)",
            r#"download.file(r"(https://example.org/data.zip)", y)"#,
            r#"download.file("https://example.org/data\x2ezip", y)"#,
            "download.file('https://example.org/data%2E%7Aip', y)",
            "download.file('https://example.org/data%FF.zip', y)",
        ] {
            expect_no_lint(code, "download_file", None);
        }

        // R's automatic binary-mode suffixes, including case-sensitive RData.
        for extension in [
            "gz", "bz2", "xz", "tgz", "zip", "jar", "rda", "rds", "RData",
        ] {
            expect_no_lint(
                &format!("download.file('https://example.org/data.{extension}', y)"),
                "download_file",
                None,
            );
        }
    }

    #[test]
    fn test_lint_download_file() {
        // Missing mode is checked for unqualified and utils-qualified calls.
        assert_snapshot!(snapshot_lint("download.file(x, y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y)
          | ------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("utils::download.file(x, y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | utils::download.file(x, y)
          | -------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // Text modes are checked by value, including raw strings, escapes, and parentheses.
        assert_snapshot!(snapshot_lint("download.file(x, y, mode = 'a')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, mode = 'a')
          | ------------------------------- `download.file()` with `mode = 'a'` can cause portability issues on Windows.
          |
          = help: Use mode = 'ab' instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(r#"download.file(x, y, mode = "w")"#), @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, mode = "w")
          | ------------------------------- `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint(r#"download.file(x, y, mode = ((r"(w)")))"#), @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, mode = ((r"(w)")))
          | -------------------------------------- `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint(r#"download.file(x, y, mode = "\x77")"#), @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, mode = "\x77")
          | ---------------------------------- `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint(r#"download.file(x, y, mode = "\141")"#), @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, mode = "\141")
          | ---------------------------------- `download.file()` with `mode = 'a'` can cause portability issues on Windows.
          |
          = help: Use mode = 'ab' instead.
        Found 1 error.
        "#);
        // Positional and mixed named/positional arguments use R argument matching.
        assert_snapshot!(snapshot_lint("download.file(x, y, z, FALSE, 'a')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, z, FALSE, 'a')
          | ---------------------------------- `download.file()` with `mode = 'a'` can cause portability issues on Windows.
          |
          = help: Use mode = 'ab' instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("download.file(x, y, FALSE, method = 'libcurl', 'w')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, FALSE, method = 'libcurl', 'w')
          | --------------------------------------------------- `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // Explicit text mode overrides automatic binary-mode detection.
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data.zip', y, mode = 'w')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data.zip', y, mode = 'w')
          | ------------------------------------------------------------ `download.file()` with `mode = 'w'` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // PDF URLs still require an explicit binary mode.
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data.pdf', y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data.pdf', y)
          | ------------------------------------------------ `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // Non-UTF-8 URL bytes do not prevent checking the suffix.
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data%FF.pdf', y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data%FF.pdf', y)
          | --------------------------------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // The suffix must match the end of the decoded URL, with the documented case.
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data.ZIP', y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data.ZIP', y)
          | ------------------------------------------------ `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data.zip?token=1', y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data.zip?token=1', y)
          | -------------------------------------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data.zip.bak', y)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data.zip.bak', y)
          | ---------------------------------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // Only the URL matters, not the destination filename.
        assert_snapshot!(snapshot_lint("download.file(x, 'data.zip')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, 'data.zip')
          | ---------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // An empty mode argument uses the default, just like an omitted one.
        assert_snapshot!(snapshot_lint("download.file(x, y, mode =)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, mode =)
          | --------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
        // libcurl honors mode; it is distinct from the external curl command.
        assert_snapshot!(snapshot_lint("download.file(x, y, method = 'libcurl')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, method = 'libcurl')
          | --------------------------------------- `download.file()` without explicit `mode` can cause portability issues on Windows.
          |
          = help: Use mode = 'wb' instead.
        Found 1 error.
        ");
    }

    #[test]
    fn test_download_file_ignored_mode() {
        // Even binary modes and dynamic expressions are ignored by external methods.
        assert_snapshot!(snapshot_lint("download.file(x, y, method = 'curl', mode = 'wb')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, method = 'curl', mode = 'wb')
          | ------------------------------------------------- `download.file()` ignores `mode` with `method = 'curl'` or `method = 'wget'`.
          |
          = help: Remove the mode argument.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("download.file(x, y, method = 'wget', mode = chosen_mode)"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, method = 'wget', mode = chosen_mode)
          | -------------------------------------------------------- `download.file()` ignores `mode` with `method = 'curl'` or `method = 'wget'`.
          |
          = help: Remove the mode argument.
        Found 1 error.
        ");
        // A named method shifts the positional mode argument.
        assert_snapshot!(snapshot_lint("download.file(x, y, FALSE, method = 'curl', 'a')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, FALSE, method = 'curl', 'a')
          | ------------------------------------------------ `download.file()` ignores `mode` with `method = 'curl'` or `method = 'wget'`.
          |
          = help: Remove the mode argument.
        Found 1 error.
        ");
        // The method is also read through parentheses and string escapes.
        assert_snapshot!(snapshot_lint(r#"download.file(x, y, method = (("\x63url")), mode = "w")"#), @r#"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file(x, y, method = (("\x63url")), mode = "w")
          | ------------------------------------------------------- `download.file()` ignores `mode` with `method = 'curl'` or `method = 'wget'`.
          |
          = help: Remove the mode argument.
        Found 1 error.
        "#);
        // URL suffix exemptions do not apply to an explicitly ignored mode.
        assert_snapshot!(snapshot_lint("download.file('https://example.org/data.zip', y, method = 'wget', mode = 'w')"), @"
        warning: download_file
         --> <test>:1:1
          |
        1 | download.file('https://example.org/data.zip', y, method = 'wget', mode = 'w')
          | ----------------------------------------------------------------------------- `download.file()` ignores `mode` with `method = 'curl'` or `method = 'wget'`.
          |
          = help: Remove the mode argument.
        Found 1 error.
        ");
    }
}
