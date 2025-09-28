use assert_cmd::cargo::CommandCargoExt;
use regex::Regex;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::Builder;

pub fn has_lint(text: &str, msg: &str, rule: &str, min_r_version: Option<&str>) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let output = if let Some(min_r_version) = min_r_version {
        Command::cargo_bin("flir")
            .unwrap()
            .arg(temp_file.path())
            .arg("--select-rules")
            .arg(rule)
            .arg("--min-r-version")
            .arg(min_r_version)
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command")
    } else {
        Command::cargo_bin("flir")
            .unwrap()
            .arg(temp_file.path())
            .arg("--select-rules")
            .arg(rule)
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command")
    };

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    let re = Regex::new(r"[A-Za-z0-9]+\.R").unwrap();
    let lint_text = re.replace_all(&lint_text, "[...]");
    lint_text.contains(msg)
}

pub fn get_fixed_text(text: Vec<&str>, rule: &str, min_r_version: Option<&str>) -> String {
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

        let _ = if let Some(min_r_version) = min_r_version {
            Command::cargo_bin("flir")
                .unwrap()
                .arg(temp_file.path())
                .arg("--fix")
                .arg("--select-rules")
                .arg(rule)
                .arg("--min-r-version")
                .arg(min_r_version)
                .stdout(Stdio::piped())
                .output()
                .expect("Failed to execute command")
        } else {
            Command::cargo_bin("flir")
                .unwrap()
                .arg(temp_file.path())
                .arg("--fix")
                .arg("--select-rules")
                .arg(rule)
                .stdout(Stdio::piped())
                .output()
                .expect("Failed to execute command")
        };

        let modified_content = fs::read_to_string(temp_file).expect("Failed to read file content");

        output.push_str(
            format!("\n\n  OLD:\n  ====\n{original_content}\n  NEW:\n  ====\n{modified_content}")
                .as_str(),
        );
    }
    output
}

pub fn get_unsafe_fixed_text(text: Vec<&str>, rule: &str) -> String {
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

        let _ = Command::cargo_bin("flir")
            .unwrap()
            .arg(temp_file.path())
            .arg("--select-rules")
            .arg(rule)
            .arg("--fix")
            .arg("--unsafe-fixes")
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let modified_content = fs::read_to_string(temp_file).expect("Failed to read file content");

        output.push_str(
            format!("\n\n  OLD:\n  ====\n{original_content}\n  NEW:\n  ====\n{modified_content}")
                .as_str(),
        );
    }
    output
}

pub fn no_lint(text: &str, rule: &str, min_r_version: Option<&str>) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    let original_content = text;

    fs::write(&temp_file, original_content).expect("Failed to write initial content");

    let _ = if let Some(min_r_version) = min_r_version {
        Command::cargo_bin("flir")
            .unwrap()
            .arg(temp_file.path())
            .arg("--select-rules")
            .arg(rule)
            .arg("--min-r-version")
            .arg(min_r_version)
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command")
    } else {
        Command::cargo_bin("flir")
            .unwrap()
            .arg(temp_file.path())
            .arg("--select-rules")
            .arg(rule)
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command")
    };

    let output = Command::cargo_bin("flir")
        .unwrap()
        .arg(temp_file.path())
        .arg("--select-rules")
        .arg(rule)
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    let lint_text = String::from_utf8_lossy(&output.stdout).to_string();
    lint_text == "All checks passed!\n"
}

pub fn expect_no_lint(text: &str, rule: &str, min_r_version: Option<&str>) {
    assert!(no_lint(text, rule, min_r_version));
}

pub fn expect_lint(text: &str, msg: &str, rule: &str, min_r_version: Option<&str>) {
    assert!(has_lint(text, msg, rule, min_r_version));
}

pub fn expect_error(text: &str, msg: &str, rule: &str) {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let output = Command::cargo_bin("flir")
        .unwrap()
        .arg(temp_file.path())
        .arg("--select-rules")
        .arg(rule)
        .stdout(Stdio::piped())
        .output()
        .unwrap()
        .stderr;

    let err_msg = std::str::from_utf8(&output).unwrap();
    assert!(err_msg.contains(msg))
}
