use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

// ---------------------------------------------------------------------------
// CLI (--assignment is deprecated, so these always emit a deprecation warning)
// ---------------------------------------------------------------------------

#[test]
fn test_assignment_from_cli() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .arg("--assignment")
                        .arg("<-")
                        .run()
                        .normalize_os_executable_name(),
                    @r"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:2:1
  |
2 | x = 1
  | --- Use `<-` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `<-` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
Warning: `--assignment` is deprecated. Use `[lint.assignment]` in jarl.toml instead.
"
                );

    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .arg("--assignment")
                        .arg("=")
                        .run()
                        .normalize_os_executable_name(),
                    @r"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:3:1
  |
3 | y <- 2
  | ---- Use `=` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `=` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
Warning: `--assignment` is deprecated. Use `[lint.assignment]` in jarl.toml instead.
"
                );

    Ok(())
}

#[test]
fn test_assignment_wrong_value_from_cli() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .arg("--assignment")
                        .arg("foo")
                        .run()
                        .normalize_os_executable_name(),
                    @r"success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Invalid value in `--assignment`: foo
"
                );

    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .arg("--assignment")
                        .arg("1")
                        .run()
                        .normalize_os_executable_name(),
                    @r"success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Invalid value in `--assignment`: 1
"
                );

    Ok(())
}

// ---------------------------------------------------------------------------
// TOML — new [lint.assignment] table syntax
// ---------------------------------------------------------------------------

#[test]
fn test_assignment_from_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint.assignment]
operator = "<-"
"#,
    )?;
    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .run()
                        .normalize_os_executable_name(),
                    @r"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:2:1
  |
2 | x = 1
  | --- Use `<-` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `<-` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
                );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint.assignment]
operator = "="
"#,
    )?;
    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .run()
                        .normalize_os_executable_name(),
                    @r"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:3:1
  |
3 | y <- 2
  | ---- Use `=` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `=` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
                );

    Ok(())
}

#[test]
fn test_assignment_wrong_value_from_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint.assignment]
operator = "foo"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
Invalid value for `operator` in `[lint.assignment]`: "foo". Expected "<-" or "=".
"#
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint.assignment]
operator = 1
"#,
    )?;
    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .run()
                        .normalize_os_executable_name()
                        .normalize_temp_paths(),
                    @r"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Failed to parse [TEMP_DIR]/jarl.toml:
TOML parse error at line 3, column 12
  |
3 | operator = 1
  |            ^
invalid type: integer `1`, expected a string

"
                );

    Ok(())
}

// ---------------------------------------------------------------------------
// CLI overrides TOML (new syntax)
// ---------------------------------------------------------------------------

#[test]
fn test_assignment_cli_overrides_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint.assignment]
operator = "<-"
"#,
    )?;
    insta::assert_snapshot!(
                    &mut Command::new(binary_path())
                        .current_dir(directory)
                        .arg("check")
                        .arg(".")
                        .arg("--select")
                        .arg("assignment")
                        .arg("--assignment")
                        .arg("=")
                        .run()
                        .normalize_os_executable_name(),
                    @r"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:3:1
  |
3 | y <- 2
  | ---- Use `=` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `=` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
Warning: `--assignment` is deprecated. Use `[lint.assignment]` in jarl.toml instead.
"
                );
    Ok(())
}

// ---------------------------------------------------------------------------
// Deprecated TOML syntax: assignment = "..." (top-level string)
// ---------------------------------------------------------------------------

#[test]
fn test_assignment_from_toml_deprecated() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "<-"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:2:1
  |
2 | x = 1
  | --- Use `<-` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `<-` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
Warning: `assignment = "..."` in `[lint]` is deprecated. Use `[lint.assignment]` with `operator = "..."` instead.
"#
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "="
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:3:1
  |
3 | y <- 2
  | ---- Use `=` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `=` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
Warning: `assignment = "..."` in `[lint]` is deprecated. Use `[lint.assignment]` with `operator = "..."` instead.
"#
    );

    Ok(())
}

#[test]
fn test_assignment_wrong_value_from_toml_deprecated() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "foo"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
Invalid value for `operator` in `[lint.assignment]`: "foo". Expected "<-" or "=".
"#
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = 1
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Failed to parse [TEMP_DIR]/jarl.toml:
TOML parse error at line 3, column 14
  |
3 | assignment = 1
  |              ^
invalid type: integer `1`, expected a string (e.g. `assignment = "<-"`) or a table (e.g. `[lint.assignment]`)

"#
    );

    Ok(())
}

#[test]
fn test_assignment_cli_overrides_toml_deprecated() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "<-"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--assignment")
            .arg("=")
            .run()
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
warning: assignment
 --> test.R:3:1
  |
3 | y <- 2
  | ---- Use `=` for assignment.
  |

warning: assignment
 --> test.R:4:3
  |
4 | 3 -> z
  |   ---- Use `=` for assignment.
  |

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
Warning: `--assignment` is deprecated. Use `[lint.assignment]` in jarl.toml instead.
Warning: `assignment = "..."` in `[lint]` is deprecated. Use `[lint.assignment]` with `operator = "..."` instead.
"#
    );
    Ok(())
}
