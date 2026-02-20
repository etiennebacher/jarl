use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_output_default() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: any_duplicated
 --> test2.R:1:1
  |
1 | any(duplicated(x))
  | ------------------ `any(duplicated(...))` is inefficient.
  |
  = help: Use `anyDuplicated(...) > 0` instead.

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_output_concise() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("concise")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
test.R [1:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test2.R [1:1] any_duplicated `any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_output_full() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("full")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: any_duplicated
 --> test2.R:1:1
  |
1 | any(duplicated(x))
  | ------------------ `any(duplicated(...))` is inefficient.
  |
  = help: Use `anyDuplicated(...) > 0` instead.

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_output_json() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("json")
            .run()
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
{
  "diagnostics": [
    {
      "message": {
        "name": "any_is_na",
        "body": "`any(is.na(...))` is inefficient.",
        "suggestion": "Use `anyNA(...)` instead."
      },
      "filename": "test.R",
      "range": [
        0,
        13
      ],
      "location": {
        "row": 1,
        "column": 0
      },
      "fix": {
        "content": "anyNA(x)",
        "start": 0,
        "end": 13,
        "to_skip": false
      }
    },
    {
      "message": {
        "name": "any_duplicated",
        "body": "`any(duplicated(...))` is inefficient.",
        "suggestion": "Use `anyDuplicated(...) > 0` instead."
      },
      "filename": "test2.R",
      "range": [
        0,
        18
      ],
      "location": {
        "row": 1,
        "column": 0
      },
      "fix": {
        "content": "anyDuplicated(x) > 0",
        "start": 0,
        "end": 18,
        "to_skip": false
      }
    }
  ],
  "errors": []
}
----- stderr -----
"#
    );

    // Additional info such as timing isn't included in output, #254
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("json")
            .arg("--with-timing")
            .run()
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
{
  "diagnostics": [
    {
      "message": {
        "name": "any_is_na",
        "body": "`any(is.na(...))` is inefficient.",
        "suggestion": "Use `anyNA(...)` instead."
      },
      "filename": "test.R",
      "range": [
        0,
        13
      ],
      "location": {
        "row": 1,
        "column": 0
      },
      "fix": {
        "content": "anyNA(x)",
        "start": 0,
        "end": 13,
        "to_skip": false
      }
    },
    {
      "message": {
        "name": "any_duplicated",
        "body": "`any(duplicated(...))` is inefficient.",
        "suggestion": "Use `anyDuplicated(...) > 0` instead."
      },
      "filename": "test2.R",
      "range": [
        0,
        18
      ],
      "location": {
        "row": 1,
        "column": 0
      },
      "fix": {
        "content": "anyDuplicated(x) > 0",
        "start": 0,
        "end": 18,
        "to_skip": false
      }
    }
  ],
  "errors": []
}
----- stderr -----
"#
    );

    Ok(())
}

#[test]
fn test_output_github() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("github")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
::warning title=Jarl (any_is_na),file=test.R,line=1,col=1::test.R:1:1 [any_is_na] `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
::warning title=Jarl (any_duplicated),file=test2.R,line=1,col=1::test2.R:1:1 [any_duplicated] `any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.

----- stderr -----
"
                        );

    // Additional info such as timing isn't included in output, #254
    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("github")
                                .arg("--with-timing")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
::warning title=Jarl (any_is_na),file=test.R,line=1,col=1::test.R:1:1 [any_is_na] `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
::warning title=Jarl (any_duplicated),file=test2.R,line=1,col=1::test2.R:1:1 [any_duplicated] `any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_with_parsing_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("full")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 255
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

Found 1 error.
1 fixable with the `--fix` option.

----- stderr -----
Error: Failed to parse test2.R due to syntax errors.

"
                        );

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("concise")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 255
----- stdout -----
test.R [1:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.

Found 1 error.
1 fixable with the `--fix` option.

----- stderr -----
Error: Failed to parse test2.R due to syntax errors.
"
                        );

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("json")
            .run()
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 255
----- stdout -----
{
  "diagnostics": [
    {
      "message": {
        "name": "any_is_na",
        "body": "`any(is.na(...))` is inefficient.",
        "suggestion": "Use `anyNA(...)` instead."
      },
      "filename": "test.R",
      "range": [
        0,
        13
      ],
      "location": {
        "row": 1,
        "column": 0
      },
      "fix": {
        "content": "anyNA(x)",
        "start": 0,
        "end": 13,
        "to_skip": false
      }
    }
  ],
  "errors": [
    {
      "file": "test2.R",
      "error": "Failed to get checks for file: test2.R: Failed to parse test2.R due to syntax errors."
    }
  ]
}
----- stderr -----
"#
    );

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("github")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 255
----- stdout -----
::warning title=Jarl (any_is_na),file=test.R,line=1,col=1::test.R:1:1 [any_is_na] `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.

----- stderr -----
"
                        );

    Ok(())
}
