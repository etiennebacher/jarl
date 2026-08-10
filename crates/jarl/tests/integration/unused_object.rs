use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_exported_alias_not_flagged() -> anyhow::Result<()> {
    // `summarize_each <- summarise_each` is a typical alias-style export. The
    // RHS isn't a function literal, so the existing function-def filter
    // doesn't suppress it; we rely on the NAMESPACE export list instead.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", "export(summarize_each)\n"),
        (
            "R/aliases.R",
            "summarise_each <- function(x) x\nsummarize_each <- summarise_each\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}
