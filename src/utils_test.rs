use regex::Regex;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::Builder;
use tempfile::NamedTempFile;

pub fn get_lint_and_fix_text(text: Vec<&str>, rule: &str) -> (String, String) {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    let separate_lint_text = text
        .iter()
        .map(|x| {
            fs::write(&temp_file, x).expect("Failed to write initial content");
            get_lint_text(&temp_file, rule)
        })
        .collect::<Vec<String>>();

    let separate_fixed_text = text
        .iter()
        .map(|x| {
            fs::write(&temp_file, x).expect("Failed to write initial content");
            get_fixed_text(&temp_file, rule)
        })
        .collect::<Vec<String>>();

    (
        separate_lint_text.join("\n\n"),
        separate_fixed_text.join("\n\n"),
    )
}

pub fn get_lint_text(file: &NamedTempFile, rule: &str) -> String {
    let original_content = fs::read_to_string(&file).expect("Failed to read file content");
    let output = Command::new("flir")
        .arg("--dir")
        .arg(file.path())
        .arg("--rules")
        .arg(rule)
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    let re = Regex::new(r"[A-Za-z0-9]+\.R").unwrap();
    let lint_text = re.replace_all(&lint_text, "[...]");

    format!(
        "  OLD:\n  ====\n{}\n  NEW:\n  ====\n{}",
        original_content, lint_text
    )
}

pub fn get_fixed_text(file: &NamedTempFile, rule: &str) -> String {
    use std::process::{Command, Stdio};
    let original_content = fs::read_to_string(&file).expect("Failed to read file content");

    let _ = Command::new("flir")
        .arg("--dir")
        .arg(file.path())
        .arg("--rules")
        .arg(rule)
        .arg("--fix")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let modified_content = fs::read_to_string(file).expect("Failed to read file content");

    format!(
        "  OLD:\n  ====\n{}\n  NEW:\n  ====\n{}",
        original_content, modified_content
    )
}

pub fn no_lint(text: &str, rule: &str) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    let original_content = text;

    fs::write(&temp_file, original_content).expect("Failed to write initial content");

    let output = Command::new("flir")
        .arg("--dir")
        .arg(temp_file.path())
        .arg("--rules")
        .arg(rule)
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    lint_text == ""
}
