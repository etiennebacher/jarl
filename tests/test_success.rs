mod utils;

use assert_cmd::Command;
use insta::assert_snapshot;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::{TempDir, tempdir};

use crate::utils::{with_test_env, write_to_r_file};

#[test]
fn test_no_r_files() -> anyhow::Result<()> {
    // Create a directory inside of `env::temp_dir()`.
    let dir = tempdir()?;

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    let mut cmd = Command::cargo_bin("flir").unwrap();
    let result = cmd.output().unwrap();

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_snapshot!("test_no_r_files", stdout);

    // By closing the `TempDir` explicitly, we can check that it has
    // been deleted successfully. If we don't close it explicitly,
    // the directory will still be deleted when `dir` goes out
    // of scope, but we won't know whether deleting the directory
    // succeeded.
    dir.close()?;
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

#[test]
fn test_no_lints() -> anyhow::Result<()> {
    // Create a directory inside of `env::temp_dir()`.
    let dir = tempdir()?;

    let file_path = dir.path().join("test.R");
    let mut file = File::create(file_path)?;
    writeln!(file, "any(x)")?;
    let original_dir = env::current_dir().unwrap();

    env::set_current_dir(dir.path()).unwrap();
    let mut cmd = Command::cargo_bin("flir").unwrap();
    let result = cmd.output().unwrap();

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert_snapshot!("test_no_r_files", stdout);

    // By closing the `TempDir` explicitly, we can check that it has
    // been deleted successfully. If we don't close it explicitly,
    // the directory will still be deleted when `dir` goes out
    // of scope, but we won't know whether deleting the directory
    // succeeded.
    drop(file);
    dir.close()?;
    env::set_current_dir(original_dir).unwrap();
    Ok(())
}

// #[test]
// fn test_no_lints() {
//     with_test_env(|temp_dir| {
//         println!("[no_lints] Temp dir: {}", temp_dir.path().display());

//         // Create and verify test file
//         let temp_file = write_to_r_file(temp_dir, "any(x)");
//         println!("Created file: {}", temp_file.display());

//         let mut cmd = Command::cargo_bin("flir").unwrap();
//         let result = cmd.output().unwrap();

//         let stdout = String::from_utf8_lossy(&result.stdout);
//         assert_snapshot!("test_no_lints", stdout);
//     });
// }

// #[test]
// fn test_one_lint() {
//     with_test_env(|temp_dir| {
//         let _temp_file = write_to_r_file(temp_dir, "any(is.na(x))");

//         let mut cmd = Command::cargo_bin("flir").unwrap();
//         let result = cmd.output().unwrap();

//         let stdout = String::from_utf8_lossy(&result.stdout);
//         assert_snapshot!("test_one_lint", stdout);
//     });
// }
