use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

// ---------------------------------------------------------------------------
// Basic lint detection
// ---------------------------------------------------------------------------

/// A lint inside an R chunk should be reported with the correct line number in
/// the original Rmd file.
#[test]
fn test_rmd_basic_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Lines 1-4: YAML front matter + blank. Line 5: fence. Line 6: code.
    std::fs::write(
        directory.join("test.Rmd"),
        "---\ntitle: \"Test\"\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

/// Same as above but with a `.qmd` extension.
#[test]
fn test_qmd_basic_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.qmd"),
        "---\ntitle: \"Test\"\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Chunk suppression
// ---------------------------------------------------------------------------

/// `#| jarl-ignore-chunk` without a rule fires `blanket_suppression` and does
/// NOT silence `any_is_na`.  A rule name and reason are required.
#[test]
fn test_rmd_ignore_chunk_suppresses() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.Rmd"),
        "```{r}\n#| jarl-ignore-chunk\nany(is.na(x))\n```\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

/// `#| jarl-ignore-chunk rule: reason` suppresses that rule for the following
/// node, just like `# jarl-ignore rule: reason`.
#[test]
fn test_rmd_ignore_chunk_with_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.Rmd"),
        "```{r}\n#| jarl-ignore-chunk any_is_na: legacy code\nany(is.na(x))\n```\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

/// `#| jarl-ignore-chunk` without a rule fires `blanket_suppression` and does
/// not silence `any_is_na`, regardless of its position in the chunk.
#[test]
fn test_rmd_ignore_chunk_not_leading() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Line 5: fence, 6: code, 7: #| jarl-ignore-chunk, 8: violation.
    std::fs::write(
        directory.join("test.Rmd"),
        "---\ntitle: \"Test\"\n---\n\n```{r}\nx <- 1\n#| jarl-ignore-chunk\nany(is.na(x))\n```\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Per-rule suppression (`#| jarl-ignore rule: reason`)
// ---------------------------------------------------------------------------

/// `#| jarl-ignore rule: reason` suppresses that rule for the following node.
#[test]
fn test_rmd_pipe_suppression() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.Rmd"),
        "```{r}\n#| jarl-ignore any_is_na: legacy code\nany(is.na(x))\n```\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// No autofix
// ---------------------------------------------------------------------------

/// Running `--fix --allow-no-vcs` on an Rmd file must not modify it.
#[test]
fn test_rmd_fix_not_applied() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let original = "```{r}\nany(is.na(x))\n```\n";
    std::fs::write(directory.join("test.Rmd"), original)?;

    // Run with --fix; redirects to lint_only for Rmd, so file is unchanged.
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg("test.Rmd")
            .arg("--fix")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    let after = std::fs::read_to_string(directory.join("test.Rmd"))?;
    assert_eq!(after, original, "Rmd file must not be modified by --fix");

    Ok(())
}
