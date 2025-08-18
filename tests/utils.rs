use std::{env, fs};
use tempfile::{Builder, TempDir, TempPath};

pub fn write_to_r_file(temp_dir: &TempDir, content: &str) -> TempPath {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile_in(&temp_dir)
        .unwrap();

    fs::write(&temp_file, content).expect("Failed to write initial content");

    temp_file.into_temp_path()
}

pub fn with_test_env<F: FnOnce(&TempDir)>(f: F) {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();

    env::set_current_dir(temp_dir.path()).unwrap();
    f(&temp_dir);
    env::set_current_dir(original_dir).unwrap();
}
