//! Integration tests for flir_lsp
//!
//! These tests verify the core functionality of the LSP server

use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn test_server_binary_exists_and_runs() {
    // Test if we can at least run the binary with --help
    let output = Command::new(env!("CARGO_BIN_EXE_flir-lsp"))
        .arg("--help")
        .output();

    match output {
        Ok(output) => {
            assert!(output.status.success(), "Binary should run with --help");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("Flir Language Server"),
                "Help should mention Flir Language Server"
            );
        }
        Err(e) => {
            panic!("Failed to run binary: {e}");
        }
    }
}

#[test]
fn test_server_startup_basic() {
    // Test if the server can start without immediate crash
    let mut child = Command::new(env!("CARGO_BIN_EXE_flir-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server process");

    // Give it a moment to start
    std::thread::sleep(Duration::from_millis(100));

    // Check if it's still running (not crashed immediately)
    let still_running = match child.try_wait() {
        Ok(Some(_)) => false, // Process exited
        Ok(None) => true,     // Still running
        Err(_) => false,      // Error checking status
    };

    // Clean up
    let _ = child.kill();
    let _ = child.wait();

    assert!(
        still_running,
        "Server should start and stay running briefly"
    );
}
