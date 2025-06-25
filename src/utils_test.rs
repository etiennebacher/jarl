use regex::Regex;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::Builder;

pub fn has_lint(text: &str, msg: &str, rule: &str) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let output = Command::new("flir")
        .arg("--dir")
        .arg(temp_file.path())
        .arg("--rules")
        .arg(rule)
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    let re = Regex::new(r"[A-Za-z0-9]+\.R").unwrap();
    let lint_text = re.replace_all(&lint_text, "[...]");
    lint_text.contains(msg)
}

pub fn get_fixed_text(text: Vec<&str>, rule: &str) -> String {
    use std::process::{Command, Stdio};

    let mut output: String = "".to_string();

    for txt in text.iter() {
        let temp_file = Builder::new()
            .prefix("test-flir")
            .suffix(".R")
            .tempfile()
            .unwrap();

        let original_content = txt;

        fs::write(&temp_file, original_content).expect("Failed to write initial content");

        let _ = Command::new("flir")
            .arg("--dir")
            .arg(temp_file.path())
            .arg("--rules")
            .arg(rule)
            .arg("--fix")
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let modified_content = fs::read_to_string(temp_file).expect("Failed to read file content");

        output.push_str(
            format!(
                "\n\n  OLD:\n  ====\n{}\n  NEW:\n  ====\n{}",
                original_content, modified_content
            )
            .as_str(),
        );
    }
    output
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
    lint_text.is_empty()
}

pub fn expect_no_lint(text: &str, rule: &str) {
    assert!(no_lint(text, rule));
}

pub fn expect_lint(text: &str, msg: &str, rule: &str) {
    assert!(has_lint(text, msg, rule));
}

pub fn expect_error(text: &str, msg: &str, rule: &str) {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let output = Command::new("flir")
        .arg("--dir")
        .arg(temp_file.path())
        .arg("--rules")
        .arg(rule)
        .stdout(Stdio::piped())
        .output()
        .unwrap()
        .stderr;

    let err_msg = std::str::from_utf8(&output).unwrap();
    assert!(err_msg.contains(msg))
}
