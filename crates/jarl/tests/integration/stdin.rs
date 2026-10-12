use crate::helpers::CliTest;
use std::io::Write;
use std::process::{Output, Stdio};

fn check_stdin(case: &CliTest, code: &str, args: &[&str]) -> anyhow::Result<Output> {
    let mut child = case
        .command()
        .arg("check")
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
        &["--stdin-file-path", "R/foo.R", "--output-format", "concise"],
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
    let out = check_stdin(&case, "x <- 1\n", &["--stdin-file-path", "nope/new.R"])?;
    assert_eq!(out.status.code(), Some(0));
    Ok(())
}

#[test]
fn test_stdin_filename_rejects_fix() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    let out = check_stdin(&case, "x <- 1\n", &["--stdin-file-path", "a.R", "--fix"])?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}

#[test]
fn test_stdin_file_path_conflicts_with_files() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    let out = check_stdin(&case, "x <- 1\n", &["folder/", "--stdin-file-path", "a.R"])?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}

#[test]
fn test_stdin_respects_exclude_in_config() -> anyhow::Result<()> {
    let case = CliTest::with_file("jarl.toml", "[lint]\nexclude = [\"R/skip.R\"]\n")?;
    let out = check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "R/skip.R"])?;
    assert_eq!(out.status.code(), Some(0));
    let out = check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "R/keep.R"])?;
    assert_eq!(out.status.code(), Some(1));
    Ok(())
}

#[test]
fn test_stdin_applies_default_exclude_without_config() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    let out = check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "renv/a.R"])?;
    assert_eq!(out.status.code(), Some(0));

    let out = check_stdin(
        &case,
        "any(is.na(x))\n",
        &["--stdin-file-path", "renv/a.R", "--no-default-exclude"],
    )?;
    assert_eq!(out.status.code(), Some(1));
    Ok(())
}

#[test]
fn test_stdin_snippet_comes_from_stdin_not_disk() -> anyhow::Result<()> {
    let case = CliTest::with_file("a.R", "# on disk\n")?;
    let out = check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "a.R"])?;
    let stdout = String::from_utf8(out.stdout)?;
    let stderr = String::from_utf8(out.stderr)?;
    assert!(stdout.contains("any(is.na(x))"), "{stdout}");
    assert!(!stderr.contains("Could not read"), "{stderr}");
    Ok(())
}
