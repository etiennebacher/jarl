use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_triple_colon_on_own_package_is_not_reported() -> anyhow::Result<()> {
    // `package_with_files()` names the package "testpkg".
    let case = CliTest::package_with_files([
        ("NAMESPACE", ""),
        (
            "R/foo.R",
            "#' Foo\n#' @examples\n#' testpkg:::internal()\n#' otherpkg:::internal()\nfoo <- function() {\n  testpkg:::internal()\n  otherpkg:::internal()\n}\n",
        ),
        ("tests/testthat.R", "testthat::test_check(\"testpkg\")\n"),
        (
            "tests/testthat/test-foo.R",
            "testpkg:::internal()\notherpkg:::internal()\n",
        ),
        (
            "data-raw/script.R",
            "testpkg:::internal()\notherpkg:::internal()\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("undesirable_operator")
            .arg("--output-format")
            .arg("concise")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    R/foo.R [4:12] undesirable_operator `:::` is listed as an undesirable operator.
    R/foo.R [7:11] undesirable_operator `:::` is listed as an undesirable operator.
    data-raw/script.R [2:9] undesirable_operator `:::` is listed as an undesirable operator.
    tests/testthat/test-foo.R [2:9] undesirable_operator `:::` is listed as an undesirable operator.

    ── Summary ──────────────────────────────────────
    Found 4 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_triple_colon_outside_of_package_is_reported() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "testpkg:::internal()\n")?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("undesirable_operator")
            .arg("--output-format")
            .arg("concise")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    test.R [1:8] undesirable_operator `:::` is listed as an undesirable operator.

    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}
