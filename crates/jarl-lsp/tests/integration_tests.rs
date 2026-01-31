//! Integration tests for jarl_lsp
//!
//! These tests verify the core functionality of the LSP server

use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn test_server_binary_exists_and_runs() {
    // Test if we can at least run the binary with --help via `jarl server`
    let jarl_bin = std::env::var("CARGO_BIN_EXE_jarl").unwrap_or_else(|_| "jarl".to_string());

    let output = Command::new(jarl_bin).args(["server", "--help"]).output();

    match output {
        Ok(output) => {
            assert!(output.status.success(), "Binary should run with --help");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("language server"),
                "Help should mention language server"
            );
        }
        Err(e) => {
            panic!("Failed to run binary: {e}");
        }
    }
}

#[test]
fn test_server_startup_basic() {
    // Test if the server can start without immediate crash via `jarl server`
    let jarl_bin = std::env::var("CARGO_BIN_EXE_jarl").unwrap_or_else(|_| "jarl".to_string());

    let mut child = Command::new(jarl_bin)
        .arg("server")
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

#[test]
fn test_diagnostic_fix_serialization() {
    use jarl_lsp::lint::DiagnosticFix;
    use serde_json;

    // Test that DiagnosticFix can be properly serialized/deserialized
    // This is used when embedding fix data in LSP diagnostics
    let fix = DiagnosticFix {
        content: "x <- 1".to_string(),
        start: 0,
        end: 5,
        is_safe: true,
        rule_name: "assignment".to_string(),
        diagnostic_start: 0,
        diagnostic_end: 5,
    };

    let json_value = serde_json::to_value(&fix).unwrap();
    let deserialized: DiagnosticFix = serde_json::from_value(json_value).unwrap();

    assert_eq!(deserialized.content, fix.content);
    assert_eq!(deserialized.start, fix.start);
    assert_eq!(deserialized.end, fix.end);
    assert_eq!(deserialized.is_safe, fix.is_safe);
}

#[test]
fn test_server_capabilities_advertise_code_actions() {
    use jarl_lsp::{Client, PositionEncoding, session::Session};
    use lsp_types::{ClientCapabilities, CodeActionKind};

    // Create minimal session to test capabilities
    let client_caps = ClientCapabilities::default();
    let pos_encoding = PositionEncoding::UTF8;
    let workspace_folders = vec![];
    let (connection, _io_threads) = lsp_server::Connection::memory();
    let client = Client::new(connection.sender);

    let session = Session::new(client_caps, pos_encoding, workspace_folders, client);
    let capabilities = session.server_capabilities();

    // Verify that code actions are properly advertised
    assert!(capabilities.code_action_provider.is_some());

    if let Some(lsp_types::CodeActionProviderCapability::Options(options)) =
        capabilities.code_action_provider
    {
        assert!(options.code_action_kinds.is_some());
        let kinds = options.code_action_kinds.unwrap();

        // Should advertise quick fix support
        assert!(kinds.contains(&CodeActionKind::QUICKFIX));
    }
}

#[test]
fn test_position_encoding_basic() {
    use jarl_lsp::PositionEncoding;

    // Test that we support the expected position encodings
    let utf8 = PositionEncoding::UTF8;
    let utf16 = PositionEncoding::UTF16;

    // These should be different variants
    assert_ne!(
        std::mem::discriminant(&utf8),
        std::mem::discriminant(&utf16)
    );

    // Note: Comprehensive Unicode testing with diagnostics and quick fixes
    // is covered in server.rs tests
}
