pub(crate) mod download_file;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

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
        let expected_message = "can cause portability issues";
        expect_lint("download.file(x)", expected_message, "download_file", None);
        expect_lint(
            "download.file(x, mode = 'a')",
            expected_message,
            "download_file",
            None,
        );
        expect_lint(
            "download.file(x, mode = 'w')",
            expected_message,
            "download_file",
            None,
        );
        expect_lint(
            "download.file(x, mode = \"a\")",
            expected_message,
            "download_file",
            None,
        );
        expect_lint(
            "download.file(x, mode = \"w\")",
            expected_message,
            "download_file",
            None,
        );
        expect_lint(
            "download.file(x, y, z, w, 'a')",
            expected_message,
            "download_file",
            None,
        );
        expect_lint(
            "download.file(x, y, z, w, 'w')",
            expected_message,
            "download_file",
            None,
        );
        // Only method = "wget" / "curl" don't trigger the lint.
        expect_lint(
            "download.file(x, y, z, method = 'foo', 'a')",
            expected_message,
            "download_file",
            None,
        );
        expect_lint(
            "download.file(x, y, z, method = 'foo', 'w')",
            expected_message,
            "download_file",
            None,
        );
    }
}
