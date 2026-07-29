pub(crate) mod nonportable_path;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "nonportable_path", None)
    }

    #[test]
    fn test_lint_nonportable_path() {
        assert_snapshot!(snapshot_lint(r#"path <- "foo/bar""#), @r#"
        warning: nonportable_path
         --> <test>:1:10
          |
        1 | path <- "foo/bar"
          |          ------- Hard-coded path separators are not portable.
          |
          = help: Use `file.path()` to construct the path.
        Found 1 error.
        "#);

        assert_eq!(check_code(r#""~/foo""#, "nonportable_path", None).len(), 1);
        assert_eq!(check_code(r#""C:/foo""#, "nonportable_path", None).len(), 1);
        assert_eq!(check_code(r#""../foo""#, "nonportable_path", None).len(), 1);
        assert_eq!(
            check_code(r#""/foo/bar""#, "nonportable_path", None).len(),
            1
        );
        assert_eq!(
            check_code(r#""foo\\bar""#, "nonportable_path", None).len(),
            1
        );
        assert_eq!(
            check_code(r#"r"(foo/bar)""#, "nonportable_path", None).len(),
            1
        );
        assert_eq!(check_code(r#""a/bb""#, "nonportable_path", None).len(), 1);
        assert_eq!(
            check_code(r#""路径1/路径2/啊啊.txt""#, "nonportable_path", None).len(),
            1
        );
        assert_eq!(
            check_code(r#""资料/🚗/结果📈.txt""#, "nonportable_path", None).len(),
            1
        );
        assert_eq!(
            check_code(
                r#"sprintf("output/%s/file.txt", x)"#,
                "nonportable_path",
                None
            )
            .len(),
            1
        );
    }

    #[test]
    fn test_no_lint_nonportable_path() {
        expect_no_lint(r#""foo""#, "nonportable_path", None);
        expect_no_lint(
            r#""https://cran.r-project.org/web/packages/lintr/""#,
            "nonportable_path",
            None,
        );
        expect_no_lint(r#""1://foo/bar""#, "nonportable_path", None);
        expect_no_lint(r#""hello\nthere!""#, "nonportable_path", None);
        expect_no_lint(r#""'/foo'""#, "nonportable_path", None);
        expect_no_lint(r#""/""#, "nonportable_path", None);
        expect_no_lint(r#""~""#, "nonportable_path", None);
        expect_no_lint(r#""c:""#, "nonportable_path", None);
        expect_no_lint(r#"".""#, "nonportable_path", None);

        // These are intentionally skipped by the fixed `lax = TRUE` heuristic.
        expect_no_lint(r#""/foo""#, "nonportable_path", None);
        expect_no_lint(r#""foo/""#, "nonportable_path", None);
        expect_no_lint(r#""~/""#, "nonportable_path", None);
        expect_no_lint(r#""C:/""#, "nonportable_path", None);
        expect_no_lint(r#""../""#, "nonportable_path", None);
        expect_no_lint(r#""/a\nsdf/bar""#, "nonportable_path", None);
        expect_no_lint(r#""/as:df/bar""#, "nonportable_path", None);
        expect_no_lint(r#""aa/b""#, "nonportable_path", None);
    }

    #[test]
    fn test_no_lint_nonportable_path_contexts() {
        expect_no_lint(r#"grep("foo/bar", x)"#, "nonportable_path", None);
        expect_no_lint(r#"grep(paste0("foo/bar"), x)"#, "nonportable_path", None);
        expect_no_lint(r#"sub("foo/bar", "", x)"#, "nonportable_path", None);
        expect_no_lint(r#"regexpr("foo/bar", x)"#, "nonportable_path", None);
        expect_no_lint(
            r#"stringr::str_detect(x, "foo/bar")"#,
            "nonportable_path",
            None,
        );
        expect_no_lint(
            r#"strptime("2025/01/31", "%Y/%m/%d")"#,
            "nonportable_path",
            None,
        );
        expect_no_lint(r#"as.Date("2025/01/31")"#, "nonportable_path", None);
        expect_no_lint(r#"grepv("foo/bar", x)"#, "nonportable_path", None);
        expect_no_lint(r#"foo(pattern = "foo/bar")"#, "nonportable_path", None);
        expect_no_lint(r#"foo(format = "%Y/%m/%d")"#, "nonportable_path", None);
    }

    #[test]
    fn test_vectorized_nonportable_path() {
        assert_snapshot!(snapshot_lint(
            r#"paths <- c("foo/bar", "https://example.com/a/b", "one/two/three")"#
        ), @r#"
        warning: nonportable_path
         --> <test>:1:13
          |
        1 | paths <- c("foo/bar", "https://example.com/a/b", "one/two/three")
          |             ------- Hard-coded path separators are not portable.
          |
          = help: Use `file.path()` to construct the path.
        warning: nonportable_path
         --> <test>:1:51
          |
        1 | paths <- c("foo/bar", "https://example.com/a/b", "one/two/three")
          |                                                   ------------- Hard-coded path separators are not portable.
          |
          = help: Use `file.path()` to construct the path.
        Found 2 errors.
        "#);
    }
}
