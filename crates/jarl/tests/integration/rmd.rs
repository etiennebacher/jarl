use crate::helpers::{CliTest, CommandExt};

// ---------------------------------------------------------------------------
// Basic lint detection
// ---------------------------------------------------------------------------

/// A lint inside an R chunk should be reported with the correct line number in
/// the original Rmd file.
#[test]
fn test_rmd_basic_lint() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
---
title: \"Test\"
---

```{r}
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.Rmd:7:1
      |
    7 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// Same as above but with a `.qmd` extension.
#[test]
fn test_qmd_basic_lint() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.qmd",
        "
---
title: \"Test\"
---

```{r}
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.qmd:7:1
      |
    7 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
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
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: blanket_suppression
     --> test.Rmd:3:1
      |
    3 | #| jarl-ignore-chunk
      | -------------------- This comment isn't used by Jarl because it is missing a rule to ignore.
      |
      = help: Use targeted comments instead, e.g., `# jarl-ignore any_is_na: <reason>`.

    warning: any_is_na
     --> test.Rmd:4:1
      |
    4 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

/// The Quarto YAML array form of `jarl-ignore-chunk` suppresses the rule for
/// the entire chunk and produces no warnings.
#[test]
fn test_rmd_ignore_chunk_with_rule() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk:
#|   - any_is_na: legacy code
any(is.na(x))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

#[test]
fn test_rmd_ignore_chunk_yaml_multiple() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk:
#|   - any_is_na: legacy code
#|   - any_duplicated: legacy code
any(is.na(x))
any(duplicated(x))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

#[test]
fn test_rmd_ignore_chunk_yaml_misplaced() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
1 + 1
#| jarl-ignore-chunk:
#|   - any_is_na: legacy code
#|   - any_duplicated: legacy code
any(is.na(x))
any(duplicated(x))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

// ---------------------------------------------------------------------------
// Per-rule suppression (`#| jarl-ignore rule: reason`)
// ---------------------------------------------------------------------------

/// `#| jarl-ignore rule: reason` is not a valid suppression comment — the `#|`
/// prefix is only recognised for `jarl-ignore-chunk`.  The violation is still
/// reported.
#[test]
fn test_rmd_pipe_suppression() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore any_is_na: legacy code
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.Rmd:4:1
      |
    4 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_standard_suppression() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```
{r}
# jarl-ignore any_is_na: legacy code
any(is.na(x))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

// ---------------------------------------------------------------------------
// invalid_chunk_suppression
// ---------------------------------------------------------------------------

/// `#| jarl-ignore-chunk rule: reason` (single-line form) is invalid.
/// It fires `invalid_chunk_suppression` and does NOT suppress `any_is_na`.
#[test]
fn test_rmd_single_line_ignore_chunk_invalid() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk any_is_na: legacy
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: invalid_chunk_suppression
     --> test.Rmd:3:1
      |
    3 | #| jarl-ignore-chunk any_is_na: legacy
      | -------------------------------------- This `jarl-ignore-chunk` comment is wrongly formatted.
      |
      = help: Use the YAML array form instead:
              #| jarl-ignore-chunk:
              #|   - <rule>: <reason>

    warning: any_is_na
     --> test.Rmd:4:1
      |
    4 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

/// `# jarl-ignore-chunk rule: reason` (hash form) is also invalid.
#[test]
fn test_rmd_hash_ignore_chunk_invalid() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
# jarl-ignore-chunk any_is_na: legacy
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: invalid_chunk_suppression
     --> test.Rmd:3:1
      |
    3 | # jarl-ignore-chunk any_is_na: legacy
      | ------------------------------------- This `jarl-ignore-chunk` comment is wrongly formatted.
      |
      = help: Use the YAML array form instead:
              #| jarl-ignore-chunk:
              #|   - <rule>: <reason>

    warning: any_is_na
     --> test.Rmd:4:1
      |
    4 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

/// One of the suppressions is invalid
#[test]
fn test_wrong_rule_name() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk:
#|   - any_is_na: foo
#|   - wrong_rule: bar
#|   - any_duplicated:
any(is.na(x))
```

```{r}
# jarl-ignore any_is_na: foo
# jarl-ignore wrong_rule: bar
# jarl-ignore any_duplicated:
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: misnamed_suppression
     --> test.Rmd:5:1
      |
    5 | #|   - wrong_rule: bar
      | ---------------------- This comment isn't used by Jarl because it contains an unrecognized rule name.
      |
      = help: Check the rule name for typos.

    warning: unexplained_suppression
     --> test.Rmd:6:1
      |
    6 | #|   - any_duplicated:
      | ---------------------- This comment isn't used by Jarl because it is missing an explanation.
      |
      = help: Add an explanation after the colon, e.g., `# jarl-ignore rule: <reason>`.

    warning: misnamed_suppression
      --> test.Rmd:12:1
       |
    12 | # jarl-ignore wrong_rule: bar
       | ----------------------------- This comment isn't used by Jarl because it contains an unrecognized rule name.
       |
       = help: Check the rule name for typos.

    warning: unexplained_suppression
      --> test.Rmd:13:1
       |
    13 | # jarl-ignore any_duplicated:
       | ----------------------------- This comment isn't used by Jarl because it is missing an explanation.
       |
       = help: Add an explanation after the colon, e.g., `# jarl-ignore rule: <reason>`.


    ── Summary ──────────────────────────────────────
    Found 4 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Chunk suppression scope
// ---------------------------------------------------------------------------

/// Chunk suppression in chunk 1 must NOT suppress violations in chunk 2.
#[test]
fn test_rmd_ignore_chunk_does_not_cross() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk:
#|   - any_is_na: only in this chunk
any(is.na(x))
```

```{r}
any(is.na(y))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.Rmd:9:1
      |
    9 | any(is.na(y))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// `#| jarl-ignore-chunk:` with no following items is a blanket suppression.
#[test]
fn test_rmd_ignore_chunk_yaml_no_items_is_blanket() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
#| jarl-ignore-chunk:
any(is.na(x))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: blanket_suppression
     --> test.Rmd:3:1
      |
    3 | #| jarl-ignore-chunk:
      | --------------------- This comment isn't used by Jarl because it is missing a rule to ignore.
      |
      = help: Use targeted comments instead, e.g., `# jarl-ignore any_is_na: <reason>`.

    warning: any_is_na
     --> test.Rmd:4:1
      |
    4 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Display-only blocks
// ---------------------------------------------------------------------------

/// ```` ```r ```` without braces is a display block and should not be linted.
#[test]
fn test_rmd_display_block_not_linted() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```r
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

// ---------------------------------------------------------------------------
// Parse errors
// ---------------------------------------------------------------------------

/// A chunk with a syntax error should be silently skipped.
#[test]
fn test_rmd_parse_error_chunk_skipped() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
1 +
```

```{r}
any(is.na(x))
```
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.Rmd:7:1
      |
    7 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// File-level suppression across chunks
// ---------------------------------------------------------------------------

/// `jarl-ignore-file` in the first chunk should suppress the rule in all chunks.
#[test]
fn test_rmd_ignore_file_cross_chunk() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
# jarl-ignore-file any_is_na: whole document
any(is.na(x))
```

```{r}
any(is.na(y))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

/// `jarl-ignore-file` in first chunk (no code there) should suppress rules in
/// other chunks and must NOT trigger `outdated_suppression`.
#[test]
fn test_rmd_ignore_file_in_first_chunk_no_outdated() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
# jarl-ignore-file any_is_na: whole document
# jarl-ignore-file any_duplicated: whole document
```

```{r}
any(is.na(1))
any(duplicated(1))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

/// `jarl-ignore-file` in a non-first R chunk should trigger
/// `misplaced_file_suppression` and must NOT suppress violations.
#[test]
fn test_rmd_ignore_file_in_second_chunk_misplaced() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
1 + 1
```

```{r}
# jarl-ignore-file any_is_na: should be misplaced
any(is.na(1))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: misplaced_file_suppression
     --> test.Rmd:7:1
      |
    7 | # jarl-ignore-file any_is_na: should be misplaced
      | ------------------------------------------------- This comment isn't used by Jarl because `# jarl-ignore-file` must be at the top of the file.
      |
      = help: Move this comment to the beginning of the file, before any code.

    warning: any_is_na
     --> test.Rmd:8:1
      |
    8 | any(is.na(1))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

/// `jarl-ignore-file` after code in the first chunk is misplaced.
#[test]
fn test_rmd_ignore_file_after_code_misplaced() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
1 + 1
# jarl-ignore-file any_is_na: should be misplaced
any(is.na(1))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: misplaced_file_suppression
     --> test.Rmd:4:1
      |
    4 | # jarl-ignore-file any_is_na: should be misplaced
      | ------------------------------------------------- This comment isn't used by Jarl because `# jarl-ignore-file` must be at the top of the file.
      |
      = help: Move this comment to the beginning of the file, before any code.

    warning: any_is_na
     --> test.Rmd:5:1
      |
    5 | any(is.na(1))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

/// A Python chunk before the first R chunk does not affect validity:
/// `jarl-ignore-file` is still accepted in the first R chunk.
#[test]
fn test_rmd_ignore_file_after_python_chunk() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "`
``{python}
x = 1
```

```{r}
# jarl-ignore-file any_is_na: whole document
```

```{r}
any(is.na(1))
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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

/// Truly unused `jarl-ignore-file` must still trigger `outdated_suppression`.
#[test]
fn test_rmd_ignore_file_truly_unused_outdated() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
# jarl-ignore-file any_is_na: whole document
```

```{r}
1 + 1
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: outdated_suppression
     --> test.Rmd:3:1
      |
    3 | # jarl-ignore-file any_is_na: whole document
      | -------------------------------------------- This suppression comment is unused, no violation would be reported without it.
      |
      = help: Remove this suppression comment or verify that it's still needed.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// No autofix
// ---------------------------------------------------------------------------

/// Running `--fix --allow-no-vcs` on an Rmd file must not modify it.
#[test]
fn test_rmd_fix_not_applied() -> anyhow::Result<()> {
    let original = "```{r}
any(is.na(x))
```
";
    let case = CliTest::with_file("test.Rmd", original)?;

    // Run with --fix; redirects to lint_only for Rmd, so file is unchanged.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("test.Rmd")
            .arg("--fix")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.Rmd:2:1
      |
    2 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    let after = case.read_file("test.Rmd")?;
    assert_eq!(after, original, "Rmd file must not be modified by --fix");

    Ok(())
}

// ---------------------------------------------------------------------------
// Chunk option names not flagged as unused
// ---------------------------------------------------------------------------

/// An R object used in a non-R chunk option (e.g. `eval=cond`) should not be
/// flagged as unused.
#[test]
fn test_rmd_chunk_option_name_not_unused() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
cond <- TRUE
```

```{bash, eval=cond}
echo 'hi'
```",
    )?;

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

/// Same for the other chunk option syntax
#[test]
fn test_qmd_chunk_option_name_not_unused() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
cond <- TRUE
```

```{bash}
#| eval: !expr cond
echo 'hi'
```",
    )?;

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

/// An R object used in an R chunk option should also not be flagged.
#[test]
fn test_rmd_r_chunk_option_name_not_unused() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "
```{r}
should_run <- TRUE
```

```{r, eval=should_run}
print('hello')
```",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
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
