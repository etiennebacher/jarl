use crate::helpers::CliTest;
use std::io::Write;
use std::process::{Output, Stdio};

fn check_stdin(case: &CliTest, code: &str, args: &[&str]) -> anyhow::Result<Output> {
    let mut child = case
        .command()
        .arg("check")
        .arg("-")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(code.as_bytes())?;
    Ok(child.wait_with_output()?)
}

#[test]
fn test_stdin_uses_virtual_path_and_config() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("jarl.toml", "[lint]\nselect = [\"any_is_na\"]\n"),
        ("R/foo.R", "x <- 1\n"),
    ])?;

    // The buffer is linted, not the file on disk, and `jarl.toml` applies.
    let out = check_stdin(
        &case,
        "any(is.na(x))\nany(duplicated(x))\n",
        &["--stdin-filename", "R/foo.R", "--output-format", "concise"],
    )?;
    let stdout = String::from_utf8(out.stdout)?;
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout.contains("R/foo.R [1:1]"), "{stdout}");
    assert!(stdout.contains("any_is_na"), "{stdout}");
    assert!(!stdout.contains("any_duplicated"), "{stdout}");
    Ok(())
}

#[test]
fn test_stdin_virtual_file_does_not_need_to_exist() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    let out = check_stdin(&case, "x <- 1\n", &["--stdin-filename", "nope/new.R"])?;
    assert_eq!(out.status.code(), Some(0));
    Ok(())
}

#[test]
fn test_stdin_requires_filename() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    let out = check_stdin(&case, "x <- 1\n", &[])?;
    assert_eq!(out.status.code(), Some(255));
    assert!(String::from_utf8(out.stderr)?.contains("--stdin-filename"));
    Ok(())
}

#[test]
fn test_stdin_filename_rejects_fix() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    let out = check_stdin(&case, "x <- 1\n", &["--stdin-filename", "a.R", "--fix"])?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}
