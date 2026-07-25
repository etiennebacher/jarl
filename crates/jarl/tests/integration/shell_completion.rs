use std::ffi::OsString;

use clap::CommandFactory;
use jarl::args::Args;

use crate::helpers::{CliTest, CommandExt};

/// Candidates offered for the argument at `index`, as the shell would get them.
fn complete(args: &[&str], index: usize) -> Vec<String> {
    complete_os(args.iter().map(OsString::from).collect(), index)
}

/// Like [complete], for arguments that can't be written as a `str`.
fn complete_os(args: Vec<OsString>, index: usize) -> Vec<String> {
    clap_complete::engine::complete(&mut Args::command(), args, index, None)
        .unwrap()
        .into_iter()
        .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
        .collect()
}

/// Shells hand over whatever the user typed, which isn't necessarily text Rust can look
/// at as a `str`.
fn invalid_utf8() -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![0xff])
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        // Unpaired surrogate
        OsString::from_wide(&[0xd800])
    }
}

/// Shells get their completions by re-running `jarl` with `COMPLETE` set, so running it
/// that way must emit a script instead of linting.
#[test]
fn test_completion_registration_script() -> anyhow::Result<()> {
    let case = CliTest::new()?;

    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let output = case.command().env("COMPLETE", shell).run();

        assert!(
            output.status.success(),
            "`{shell}` registration script failed: {}",
            output.stderr
        );
        assert!(
            output.stdout.contains("jarl"),
            "`{shell}` registration script doesn't mention the `jarl` binary"
        );
    }

    Ok(())
}

#[test]
fn test_complete_commands() {
    let candidates = complete(&["jarl", ""], 1);

    assert!(candidates.contains(&"check".to_string()));
    assert!(candidates.contains(&"rule".to_string()));
    assert!(candidates.contains(&"server".to_string()));
}

#[test]
fn test_complete_rule_names() {
    let candidates = complete(&["jarl", "check", "--select", "any"], 3);

    assert_eq!(candidates, ["any_duplicated", "any_is_na"]);
}

#[test]
fn test_complete_rule_names_offers_groups() {
    let candidates = complete(&["jarl", "check", "--ignore", ""], 3);

    assert!(candidates.contains(&"ALL".to_string()));
    assert!(candidates.contains(&"PERF".to_string()));
    assert!(candidates.contains(&"any_is_na".to_string()));
}

/// The arguments take a comma-separated list, so every name but the first is completed
/// from a value that already holds the previous ones.
#[test]
fn test_complete_rule_names_after_comma() {
    let candidates = complete(&["jarl", "check", "--select", "any_is_na,cl"], 3);

    assert_eq!(candidates, ["any_is_na,class_equals"]);

    let candidates = complete(
        &["jarl", "check", "--extend-select", "PERF,any_is_na,seq"],
        3,
    );

    assert_eq!(candidates, ["PERF,any_is_na,seq", "PERF,any_is_na,seq2"]);
}

#[test]
fn test_complete_rule_names_ignores_invalid_utf8() {
    let args = vec![
        OsString::from("jarl"),
        OsString::from("check"),
        OsString::from("--select"),
        invalid_utf8(),
    ];

    assert!(complete_os(args, 3).is_empty());

    // Guards against the assertion above passing because nothing is ever completed
    assert!(!complete(&["jarl", "check", "--select", "any"], 3).is_empty());
}

#[test]
fn test_complete_option_values() {
    assert!(complete(&["jarl", "check", "--output-format", ""], 3).contains(&"sarif".to_string()));
    assert!(complete(&["jarl", "check", "--log-level", ""], 3).contains(&"trace".to_string()));
}
