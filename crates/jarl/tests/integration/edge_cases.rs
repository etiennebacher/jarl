use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

// This collects edge cases and runs them with all rules to ensure that we didn't
// fix just one particular rule but left errors in another one;

// https://github.com/etiennebacher/jarl/issues/416
#[test]
fn test_jarl_break_and_next_kw_as_call() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
for (i in 1:3) {
    break()
}
for (i in 1:3) {
    next()
}",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
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
