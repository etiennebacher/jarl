use regex::Regex;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::Builder;
use tempfile::NamedTempFile;

pub fn get_lint_and_fix_text(text: &str) -> (String, String) {
    let temp_file = Builder::new()
        .prefix("test-flint")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");
    (get_lint_text(&temp_file), get_fixed_text(&temp_file))
}

pub fn get_lint_text(file: &NamedTempFile) -> String {
    let original_content = fs::read_to_string(&file).expect("Failed to read file content");
    let output = Command::new("flint")
        .arg("--dir")
        .arg(file.path())
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    let re = Regex::new(r"[A-Za-z0-9]+\.R").unwrap();
    let lint_text = re.replace_all(&lint_text, "[...]");

    format!(
        "ORIGINAL:\n=========\n{}\n\nNEW:\n====\n{}",
        original_content, lint_text
    )
}

pub fn get_fixed_text(file: &NamedTempFile) -> String {
    use std::process::{Command, Stdio};
    let original_content = fs::read_to_string(&file).expect("Failed to read file content");

    let _ = Command::new("flint")
        .arg("--dir")
        .arg(file.path())
        .arg("--fix")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let modified_content = fs::read_to_string(file).expect("Failed to read file content");

    format!(
        "ORIGINAL:\n=========\n{}\n\nNEW:\n====\n{}",
        original_content, modified_content
    )
}

pub fn no_lint(text: &str) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flint")
        .suffix(".R")
        .tempfile()
        .unwrap();

    let original_content = text;

    fs::write(&temp_file, original_content).expect("Failed to write initial content");

    let output = Command::new("flint")
        .arg("--dir")
        .arg(temp_file.path())
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    lint_text == ""
}
