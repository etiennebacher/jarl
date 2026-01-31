//! Main LSP server implementation for Jarl
//!
//! This module contains the core server logic that handles the LSP protocol,
//! providing diagnostic (linting) capabilities and code actions for quick fixes.

use anyhow::{Context, Result, anyhow};
use crossbeam::channel;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{self as types, notification::Notification as _, request::Request as _};

use std::num::NonZeroUsize;
use std::thread;
use std::time::Instant;

use crate::LspResult;
use crate::client::{Client, ToLspError};
use crate::document::TextDocument;
use crate::lint;
use crate::session::{DocumentSnapshot, Session, negotiate_position_encoding};

/// Main LSP server
pub struct Server {
    connection: Connection,
    worker_threads: NonZeroUsize,
}

/// Events that can be processed by the main loop
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    /// LSP message from client
    Message(Message),
    /// Internal task to send a response
    SendResponse(Response),
    /// Shutdown the server
    Shutdown,
}

/// Background task that can be executed by worker threads
pub enum Task {
    /// Lint a document and publish diagnostics
    LintDocument {
        snapshot: Box<DocumentSnapshot>,
        client: Client,
    },
    /// Handle a diagnostic request
    HandleDiagnosticRequest {
        snapshot: Box<DocumentSnapshot>,
        request_id: RequestId,
        client: Client,
    },
    /// Handle a code action request
    HandleCodeActionRequest {
        snapshot: Box<DocumentSnapshot>,
        request_id: RequestId,
        params: Box<types::CodeActionParams>,
        client: Client,
    },
}

impl Server {
    /// Create a new server instance
    pub fn new(worker_threads: NonZeroUsize, connection: Connection) -> Result<Self> {
        Ok(Self { connection, worker_threads })
    }

    /// Run the main server loop
    pub fn run(self) -> Result<()> {
        tracing::info!("Starting LSP handshake");

        // Perform LSP handshake
        let (id, init_params) = self
            .connection
            .initialize_start()
            .context("Failed to start LSP initialization")?;

        tracing::debug!("Received initialize request with id: {:?}", id);

        // Parse initialize params
        let init_params: lsp_types::InitializeParams = serde_json::from_value(init_params)
            .context("Failed to parse initialization parameters")?;

        tracing::debug!("Parsed initialize params successfully");

        // Negotiate capabilities
        let client_capabilities = init_params.capabilities.clone();
        let position_encoding = negotiate_position_encoding(&client_capabilities);

        tracing::info!("Negotiated position encoding: {:?}", position_encoding);
        tracing::debug!("Position encoding negotiated: {:?}", position_encoding);

        // Create client for communication
        let client = Client::new(self.connection.sender.clone());

        // Create session
        let mut session = Session::new(
            client_capabilities,
            position_encoding,
            vec![], // Will be populated from init_params
            client.clone(),
        );

        // Initialize session and get initialize result
        let initialize_result = session
            .initialize(init_params)
            .context("Failed to initialize session")?;

        // Complete handshake
        let initialize_result_json = serde_json::to_value(initialize_result)
            .context("Failed to serialize initialize result")?;
        tracing::debug!("Initialize result: {:?}", initialize_result_json);

        self.connection
            .initialize_finish(id, initialize_result_json)
            .context("Failed to finish LSP initialization")?;
        tracing::info!("LSP server initialized successfully");

        // Create worker thread pool
        let (task_sender, task_receiver) = channel::bounded::<Task>(100);
        let (event_sender, event_receiver) = channel::bounded::<Event>(100);

        // Spawn worker threads
        tracing::debug!("Spawning {} worker threads", self.worker_threads.get());
        for i in 0..self.worker_threads.get() {
            let task_receiver = task_receiver.clone();
            let event_sender = event_sender.clone();
            thread::spawn(move || {
                tracing::debug!("Worker thread {} started", i);
                Self::worker_thread(i, task_receiver, event_sender);
                tracing::debug!("Worker thread {} stopped", i);
            });
        }

        // Run main loop
        tracing::debug!("Starting main event loop");
        self.main_loop(session, task_sender, event_receiver)
    }

    /// Main event processing loop
    fn main_loop(
        &self,
        mut session: Session,
        task_sender: channel::Sender<Task>,
        event_receiver: channel::Receiver<Event>,
    ) -> Result<()> {
        tracing::info!("Starting main event loop");

        loop {
            crossbeam::select! {
                // Handle LSP messages from client
                recv(self.connection.receiver) -> msg => {
                    match msg {
                        Ok(msg) => {
                            if let Err(e) = self.handle_message(msg, &mut session, &task_sender) {
                                tracing::error!("Error handling message: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error receiving message: {}", e);
                            break;
                        }
                    }
                }
                // Handle internal events
                recv(event_receiver) -> event => {
                    match event {
                        Ok(Event::Message(msg)) => {
                            if let Err(e) = self.handle_message(msg, &mut session, &task_sender) {
                                tracing::error!("Error handling internal message: {}", e);
                            }
                        }
                        Ok(Event::SendResponse(response)) => {
                            if let Err(e) = self.connection.sender.send(Message::Response(response)) {
                                tracing::error!("Error sending response: {}", e);
                            }
                        }
                        Ok(Event::Shutdown) => {
                            tracing::info!("Shutdown event received");
                            break;
                        }
                        Err(_) => {
                            tracing::warn!("Event channel closed");
                            break;
                        }
                    }
                }
            }

            if session.is_shutdown_requested() {
                break;
            }
        }

        tracing::info!("Main loop stopped");
        Ok(())
    }

    /// Handle an LSP message
    fn handle_message(
        &self,
        message: Message,
        session: &mut Session,
        task_sender: &channel::Sender<Task>,
    ) -> LspResult<()> {
        match message {
            Message::Request(request) => self.handle_request(request, session, task_sender),
            Message::Notification(notification) => {
                Self::handle_notification(notification, session, task_sender)
            }
            Message::Response(response) => {
                session.client().handle_response(response);
                Ok(())
            }
        }
    }

    /// Handle a request from the client
    fn handle_request(
        &self,
        request: Request,
        session: &mut Session,
        task_sender: &channel::Sender<Task>,
    ) -> LspResult<()> {
        let client = session.client().clone();

        match request.method.as_str() {
            types::request::Shutdown::METHOD => {
                session.request_shutdown();
                client.send_response(request.id, ())?;
                Ok(())
            }
            types::request::DocumentDiagnosticRequest::METHOD => {
                let params: types::DocumentDiagnosticParams =
                    serde_json::from_value(request.params)?;

                if let Some(snapshot) = session.take_snapshot(params.text_document.uri) {
                    task_sender.send(Task::HandleDiagnosticRequest {
                        snapshot: Box::new(snapshot),
                        request_id: request.id,
                        client,
                    })?;
                } else {
                    client.send_error_response(
                        request.id,
                        anyhow!("Document not found").to_lsp_error(),
                    )?;
                }
                Ok(())
            }
            types::request::CodeActionRequest::METHOD => {
                let params: types::CodeActionParams = serde_json::from_value(request.params)?;
                let uri = params.text_document.uri.clone();

                if let Some(snapshot) = session.take_snapshot(uri) {
                    task_sender.send(Task::HandleCodeActionRequest {
                        snapshot: Box::new(snapshot),
                        request_id: request.id,
                        params: Box::new(params),
                        client,
                    })?;
                } else {
                    client.send_error_response(
                        request.id,
                        anyhow!("Document not found").to_lsp_error(),
                    )?;
                }
                Ok(())
            }
            _ => {
                tracing::debug!(
                    "Unhandled request method: {} (not supported in diagnostics-only mode)",
                    request.method
                );
                client.send_error_response(
                    request.id,
                    anyhow!("Method not supported - this is a diagnostics-only LSP server")
                        .to_lsp_error_with_code(-32601),
                )?;
                Ok(())
            }
        }
    }

    /// Handle a notification from the client
    fn handle_notification(
        notification: Notification,
        session: &mut Session,
        task_sender: &channel::Sender<Task>,
    ) -> LspResult<()> {
        tracing::debug!("Handling notification: {}", notification.method);
        match notification.method.as_str() {
            types::notification::Exit::METHOD => {
                if session.is_shutdown_requested() {
                    tracing::info!("Clean shutdown requested");
                } else {
                    tracing::warn!("Exit without shutdown - this is a protocol violation");
                }
                std::process::exit(0);
            }
            types::notification::DidOpenTextDocument::METHOD => {
                let params: types::DidOpenTextDocumentParams =
                    serde_json::from_value(notification.params)?;

                tracing::debug!("Document opened: {}", params.text_document.uri);

                let document =
                    TextDocument::new(params.text_document.text, params.text_document.version)
                        .with_language_id(&params.text_document.language_id);

                session.open_document(params.text_document.uri.clone(), document);

                // Check and notify about config file location (once per session, only if not in CWD)
                if let Ok(file_path) = params.text_document.uri.to_file_path() {
                    session.check_and_notify_config(&file_path);
                }

                // Trigger linting for push diagnostics (real-time as you type)
                let supports_pull_diagnostics = session.supports_pull_diagnostics();

                if !supports_pull_diagnostics
                    && let Some(snapshot) = session.take_snapshot(params.text_document.uri)
                {
                    task_sender.send(Task::LintDocument {
                        snapshot: Box::new(snapshot),
                        client: session.client().clone(),
                    })?;
                }
                Ok(())
            }
            types::notification::DidChangeTextDocument::METHOD => {
                let params: types::DidChangeTextDocumentParams =
                    serde_json::from_value(notification.params)?;

                tracing::debug!("Document changed: {}", params.text_document.uri);

                session.update_document(
                    params.text_document.uri.clone(),
                    params.content_changes,
                    params.text_document.version,
                )?;

                // Don't trigger linting on every change, only on save
                Ok(())
            }
            types::notification::DidCloseTextDocument::METHOD => {
                let params: types::DidCloseTextDocumentParams =
                    serde_json::from_value(notification.params)?;

                session.close_document(params.text_document.uri.clone())?;

                // Clear diagnostics for the closed document
                session
                    .client()
                    .publish_diagnostics(params.text_document.uri, vec![], None)?;
                Ok(())
            }
            types::notification::DidSaveTextDocument::METHOD => {
                let params: types::DidSaveTextDocumentParams =
                    serde_json::from_value(notification.params)?;

                tracing::debug!("Document saved: {}", params.text_document.uri);

                let supports_pull_diagnostics = session.supports_pull_diagnostics();

                if !supports_pull_diagnostics
                    && let Some(snapshot) = session.take_snapshot(params.text_document.uri)
                {
                    task_sender.send(Task::LintDocument {
                        snapshot: Box::new(snapshot),
                        client: session.client().clone(),
                    })?;
                }
                Ok(())
            }
            _ => {
                tracing::debug!("Unhandled notification: {}", notification.method);
                Ok(())
            }
        }
    }

    /// Worker thread that processes background tasks
    fn worker_thread(
        _id: usize,
        task_receiver: channel::Receiver<Task>,
        event_sender: channel::Sender<Event>,
    ) {
        while let Ok(task) = task_receiver.recv() {
            match task {
                Task::LintDocument { snapshot, client } => {
                    if let Err(e) = Self::handle_lint_task(*snapshot, client) {
                        tracing::error!("Error in lint task: {}", e);
                    }
                }
                Task::HandleDiagnosticRequest { snapshot, request_id, client } => {
                    if let Err(e) = Self::handle_diagnostic_request(
                        *snapshot,
                        request_id,
                        client,
                        &event_sender,
                    ) {
                        tracing::error!("Error in diagnostic request task: {}", e);
                    }
                }
                Task::HandleCodeActionRequest { snapshot, request_id, params, client } => {
                    Self::handle_code_action_request(*snapshot, request_id, *params, client);
                }
            }
        }
    }

    /// Handle linting a document and publishing diagnostics
    fn handle_lint_task(snapshot: DocumentSnapshot, client: Client) -> LspResult<()> {
        let start = Instant::now();
        let diagnostics = lint::lint_document(&snapshot)?;
        let elapsed = start.elapsed();

        tracing::debug!(
            "Linted {} in {:?}: {} diagnostics found",
            snapshot.uri(),
            elapsed,
            diagnostics.len()
        );

        client.publish_diagnostics(
            snapshot.uri().clone(),
            diagnostics,
            Some(snapshot.version()),
        )?;
        Ok(())
    }

    /// Handle a diagnostic request
    fn handle_diagnostic_request(
        snapshot: DocumentSnapshot,
        request_id: RequestId,
        _client: Client,
        event_sender: &channel::Sender<Event>,
    ) -> LspResult<()> {
        let diagnostics = lint::lint_document(&snapshot)?;

        let result = types::DocumentDiagnosticReportResult::Report(
            types::DocumentDiagnosticReport::Full(types::RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: types::FullDocumentDiagnosticReport {
                    result_id: None,
                    items: diagnostics,
                },
            }),
        );

        let response = Response {
            id: request_id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        };

        event_sender.send(Event::SendResponse(response))?;
        Ok(())
    }

    /// Handle a code action request by providing quick fixes for diagnostics
    fn handle_code_action_request(
        snapshot: DocumentSnapshot,
        request_id: RequestId,
        params: types::CodeActionParams,
        client: Client,
    ) {
        match Self::generate_code_actions(&snapshot, &params) {
            Ok(actions) => {
                if let Err(e) = client.send_response(request_id, actions) {
                    tracing::error!("Failed to send code actions: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to generate code actions: {}", e);
                if let Err(send_err) = client.send_error_response(request_id, e.to_lsp_error()) {
                    tracing::error!("Failed to send error response: {}", send_err);
                }
            }
        }
    }

    /// Generate code actions (quick fixes) for diagnostics in the given range
    fn generate_code_actions(
        snapshot: &DocumentSnapshot,
        params: &types::CodeActionParams,
    ) -> LspResult<Vec<types::CodeActionOrCommand>> {
        use crate::lint::lint_document;

        // Get diagnostics with fix information
        let diagnostics = lint_document(snapshot)?;

        let mut actions = Vec::new();

        // Filter diagnostics that intersect with the requested range
        for diagnostic in diagnostics {
            if ranges_overlap(&diagnostic.range, &params.range) {
                // Add the regular fix action if available
                if let Some(action) = Self::diagnostic_to_code_action(&diagnostic, snapshot) {
                    actions.push(types::CodeActionOrCommand::CodeAction(action));
                }

                // Add nolint actions
                if let Some(action) = Self::diagnostic_to_nolint_rule_action(&diagnostic, snapshot)
                {
                    actions.push(types::CodeActionOrCommand::CodeAction(action));
                }

                if let Some(action) = Self::diagnostic_to_nolint_all_action(&diagnostic, snapshot) {
                    actions.push(types::CodeActionOrCommand::CodeAction(action));
                }
            }
        }

        Ok(actions)
    }

    /// Convert a diagnostic with fix information to a code action
    fn diagnostic_to_code_action(
        diagnostic: &types::Diagnostic,
        snapshot: &DocumentSnapshot,
    ) -> Option<types::CodeAction> {
        // Extract fix data from diagnostic (we'll store it in the data field)
        let fix_data = diagnostic.data.as_ref()?;
        let fix: crate::lint::DiagnosticFix = serde_json::from_value(fix_data.clone()).ok()?;

        if fix.content.is_empty() && fix.start == fix.end {
            return None; // No fix available
        }

        // Convert byte offsets to LSP positions
        let content = snapshot.content();
        let encoding = snapshot.position_encoding();

        let start_pos =
            crate::lint::byte_offset_to_lsp_position(fix.start, content, encoding).ok()?;
        let end_pos = crate::lint::byte_offset_to_lsp_position(fix.end, content, encoding).ok()?;

        let edit_range = types::Range::new(start_pos, end_pos);

        // Create the text edit for this single file
        let text_edit = types::TextEdit { range: edit_range, new_text: fix.content.clone() };

        // Create workspace edit with just this file's changes
        let mut changes = std::collections::HashMap::new();
        changes.insert(snapshot.uri().clone(), vec![text_edit]);

        let workspace_edit = types::WorkspaceEdit { changes: Some(changes), ..Default::default() };

        // Determine the fix kind based on safety
        let kind = if fix.is_safe {
            types::CodeActionKind::QUICKFIX
        } else {
            types::CodeActionKind::from("quickfix.unsafe".to_string())
        };

        Some(types::CodeAction {
            title: format!("Fix: {}", diagnostic.message),
            kind: Some(kind),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(fix.is_safe),
            disabled: None,
            data: None,
        })
    }

    /// Create a code action to add a nolint comment for a specific rule
    fn diagnostic_to_nolint_rule_action(
        diagnostic: &types::Diagnostic,
        snapshot: &DocumentSnapshot,
    ) -> Option<types::CodeAction> {
        let content = snapshot.content();

        // Extract the rule name from the diagnostic data
        let fix_data = diagnostic.data.as_ref()?;
        let fix: crate::lint::DiagnosticFix = serde_json::from_value(fix_data.clone()).ok()?;
        let rule_name = fix.rule_name;

        // Find the start of the line where the diagnostic is
        let line_start = diagnostic.range.start.line;
        let line_start_pos = types::Position::new(line_start, 0);

        // Calculate the indentation of the current line
        let line_text = Self::get_line_text(content, line_start as usize)?;
        let indent = line_text
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();

        // Check if there's already a nolint comment on the previous line
        let (insert_pos, new_comment) = if line_start > 0 {
            let prev_line_text = Self::get_line_text(content, (line_start - 1) as usize)?;
            let trimmed = prev_line_text.trim();

            // Check if previous line is a generic nolint or already contains this rule
            if trimmed == "# nolint" {
                // Generic nolint already exists, no need to add specific rule
                return None;
            }

            if let Some(updated_comment) = Self::update_existing_nolint(&prev_line_text, &rule_name)
            {
                // Update existing nolint comment (replace without newline since we're replacing the line content)
                let prev_line_start = types::Position::new(line_start - 1, 0);
                let prev_line_end =
                    types::Position::new(line_start - 1, prev_line_text.len() as u32);
                (
                    types::Range::new(prev_line_start, prev_line_end),
                    updated_comment,
                )
            } else if trimmed.starts_with("# nolint:") {
                // Rule already exists in the nolint comment (update_existing_nolint returned None)
                return None;
            } else {
                // Insert new nolint comment
                (
                    types::Range::new(line_start_pos, line_start_pos),
                    format!("{}# nolint: {}\n", indent, rule_name),
                )
            }
        } else {
            // First line, just insert
            (
                types::Range::new(line_start_pos, line_start_pos),
                format!("{}# nolint: {}\n", indent, rule_name),
            )
        };

        let text_edit = types::TextEdit { range: insert_pos, new_text: new_comment };

        let mut changes = std::collections::HashMap::new();
        changes.insert(snapshot.uri().clone(), vec![text_edit]);

        let workspace_edit = types::WorkspaceEdit { changes: Some(changes), ..Default::default() };

        Some(types::CodeAction {
            title: format!("Ignore `{}` violation on this node.", rule_name),
            kind: Some(types::CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    /// Create a code action to add a nolint comment for all rules
    fn diagnostic_to_nolint_all_action(
        diagnostic: &types::Diagnostic,
        snapshot: &DocumentSnapshot,
    ) -> Option<types::CodeAction> {
        let content = snapshot.content();

        // Find the start of the line where the diagnostic is
        let line_start = diagnostic.range.start.line;
        let line_start_pos = types::Position::new(line_start, 0);

        // Calculate the indentation of the current line
        let line_text = Self::get_line_text(content, line_start as usize)?;
        let indent = line_text
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();

        // Check if there's already a nolint comment on the previous line
        let (insert_pos, new_comment) = if line_start > 0 {
            let prev_line_text = Self::get_line_text(content, (line_start - 1) as usize)?;
            if prev_line_text.trim().starts_with("# nolint") {
                // Already has a nolint comment, replace it with the all version (no newline since we're replacing)
                let prev_line_start = types::Position::new(line_start - 1, 0);
                let prev_line_end =
                    types::Position::new(line_start - 1, prev_line_text.len() as u32);
                (
                    types::Range::new(prev_line_start, prev_line_end),
                    format!("{}# nolint", indent),
                )
            } else {
                // Insert new nolint comment
                (
                    types::Range::new(line_start_pos, line_start_pos),
                    format!("{}# nolint\n", indent),
                )
            }
        } else {
            // First line, just insert
            (
                types::Range::new(line_start_pos, line_start_pos),
                format!("{}# nolint\n", indent),
            )
        };

        let text_edit = types::TextEdit { range: insert_pos, new_text: new_comment };

        let mut changes = std::collections::HashMap::new();
        changes.insert(snapshot.uri().clone(), vec![text_edit]);

        let workspace_edit = types::WorkspaceEdit { changes: Some(changes), ..Default::default() };

        Some(types::CodeAction {
            title: "Ignore all violations on this node.".to_string(),
            kind: Some(types::CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    /// Get the text of a specific line
    fn get_line_text(content: &str, line_number: usize) -> Option<String> {
        content.lines().nth(line_number).map(|s| s.to_string())
    }

    /// Update an existing nolint comment to include a new rule
    fn update_existing_nolint(line: &str, rule_name: &str) -> Option<String> {
        let trimmed = line.trim();

        // Check if this is a nolint comment
        if !trimmed.starts_with("# nolint") {
            return None;
        }

        // If it's already a generic "# nolint", leave it as is
        if trimmed == "# nolint" {
            return None;
        }

        // Extract existing rules
        if let Some(colon_pos) = trimmed.find(':') {
            let rules_part = trimmed[colon_pos + 1..].trim();
            let existing_rules: Vec<&str> = rules_part.split(',').map(|s| s.trim()).collect();

            // Check if the rule is already there
            if existing_rules.contains(&rule_name) {
                return None;
            }

            // Add the new rule
            let indent = line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();
            let all_rules = existing_rules
                .iter()
                .chain(std::iter::once(&rule_name))
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            Some(format!("{}# nolint: {}", indent, all_rules))
        } else {
            None
        }
    }
}

/// Check if two ranges overlap
fn ranges_overlap(a: &types::Range, b: &types::Range) -> bool {
    a.start <= b.end && b.start <= a.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::PositionEncoding;
    use crate::document::{DocumentKey, TextDocument};
    use crate::lint::{self, DiagnosticFix};
    use crate::session::DocumentSnapshot;
    use lsp_server::Connection;
    use lsp_types::{Position, Range, Url};
    use tempfile::TempDir;

    const CURSOR: &str = "<CURS>";

    /// Test environment that creates a temp directory with jarl.toml config
    struct TestEnv {
        _dir: TempDir,
        file_path: std::path::PathBuf,
    }

    impl TestEnv {
        fn new(content: &str) -> Self {
            let dir = TempDir::new().unwrap();
            let dir_path = dir.path();

            // Create jarl.toml with all rules enabled including opt-in rules
            std::fs::write(
                dir_path.join("jarl.toml"),
                r#"
[lint]
default-exclude = false
select = ["ALL"]
"#,
            )
            .unwrap();

            // Create the R file
            let file_path = dir_path.join("test.R");
            std::fs::write(&file_path, content).unwrap();

            Self { _dir: dir, file_path }
        }

        fn create_snapshot(&self, content: &str) -> DocumentSnapshot {
            let uri = Url::from_file_path(&self.file_path).unwrap();
            let key = DocumentKey::from(uri);
            let document = TextDocument::new(content.to_string(), 1);

            DocumentSnapshot::new(
                document,
                key,
                PositionEncoding::UTF8,
                lsp_types::ClientCapabilities::default(),
            )
        }
    }

    fn create_test_snapshot(content: &str) -> DocumentSnapshot {
        let uri = Url::parse("file:///test.R").unwrap();
        let key = DocumentKey::from(uri);
        let document = TextDocument::new(content.to_string(), 1);

        DocumentSnapshot::new(
            document,
            key,
            PositionEncoding::UTF8,
            lsp_types::ClientCapabilities::default(),
        )
    }

    fn create_test_snapshot_with_encoding(
        content: &str,
        encoding: PositionEncoding,
    ) -> DocumentSnapshot {
        let uri = Url::parse("file:///test.R").unwrap();
        let key = DocumentKey::from(uri);
        let document = TextDocument::new(content.to_string(), 1);

        DocumentSnapshot::new(
            document,
            key,
            encoding,
            lsp_types::ClientCapabilities::default(),
        )
    }

    fn create_test_diagnostic_with_fix(
        range: Range,
        message: String,
        fix: DiagnosticFix,
    ) -> types::Diagnostic {
        let fix_data = serde_json::to_value(&fix).unwrap();

        types::Diagnostic {
            range,
            severity: Some(types::DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("jarl".to_string()),
            message,
            related_information: None,
            tags: None,
            data: Some(fix_data),
        }
    }

    /// Convert byte offset to LSP position
    fn offset_to_position(content: &str, offset: usize) -> Position {
        let mut line = 0;
        let mut col = 0;
        for (i, ch) in content.char_indices() {
            if i == offset {
                return Position::new(line, col);
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Position::new(line, col)
    }

    /// Convert LSP Position to byte offset
    fn position_to_offset(content: &str, position: Position) -> usize {
        let mut offset = 0;
        let mut current_line = 0;

        for line in content.lines() {
            if current_line == position.line {
                return offset + position.character as usize;
            }
            offset += line.len() + 1;
            current_line += 1;
        }

        if current_line == position.line {
            offset + position.character as usize
        } else {
            content.len()
        }
    }

    /// Check if a position is within a range
    fn position_in_range(pos: Position, range: &Range) -> bool {
        if pos.line < range.start.line || pos.line > range.end.line {
            return false;
        }
        if pos.line == range.start.line && pos.character < range.start.character {
            return false;
        }
        if pos.line == range.end.line && pos.character > range.end.character {
            return false;
        }
        true
    }

    /// Apply a quick fix at the cursor position by running the actual linter.
    fn apply_fix_at_cursor(source_with_cursor: &str) -> Option<String> {
        let cursor_pos = source_with_cursor.find(CURSOR)?;
        let content = source_with_cursor.replace(CURSOR, "");

        let env = TestEnv::new(&content);
        let snapshot = env.create_snapshot(&content);

        // Run the linter to get real diagnostics
        let diagnostics = lint::lint_document(&snapshot).ok()?;

        // Find the diagnostic at cursor position
        let cursor_lsp_pos = offset_to_position(&content, cursor_pos);
        let diagnostic = diagnostics
            .iter()
            .find(|d| position_in_range(cursor_lsp_pos, &d.range))?;

        // Get the code action
        let action = Server::diagnostic_to_code_action(diagnostic, &snapshot)?;
        let edit = action.edit?;
        let changes = edit.changes?;
        let text_edits = changes.values().next()?;

        // Apply edits
        let mut result = content.clone();
        for text_edit in text_edits.iter().rev() {
            let start = position_to_offset(&result, text_edit.range.start);
            let end = position_to_offset(&result, text_edit.range.end);
            result.replace_range(start..end, &text_edit.new_text);
        }

        Some(result)
    }

    /// Apply a nolint action at the cursor position by running the actual linter.
    fn apply_nolint_at_cursor(source_with_cursor: &str) -> Option<String> {
        let cursor_pos = source_with_cursor.find(CURSOR)?;
        let content = source_with_cursor.replace(CURSOR, "");

        let env = TestEnv::new(&content);
        let snapshot = env.create_snapshot(&content);

        // Run the linter to get real diagnostics
        let diagnostics = lint::lint_document(&snapshot).ok()?;

        // Find the diagnostic at cursor position
        let cursor_lsp_pos = offset_to_position(&content, cursor_pos);
        let diagnostic = diagnostics
            .iter()
            .find(|d| position_in_range(cursor_lsp_pos, &d.range))?;

        // Get the nolint action
        let action = Server::diagnostic_to_nolint_rule_action(diagnostic, &snapshot)?;
        let edit = action.edit?;
        let changes = edit.changes?;
        let text_edits = changes.values().next()?;

        // Apply edits
        let mut result = content.clone();
        for text_edit in text_edits.iter().rev() {
            let start = position_to_offset(&result, text_edit.range.start);
            let end = position_to_offset(&result, text_edit.range.end);
            result.replace_range(start..end, &text_edit.new_text);
        }

        Some(result)
    }

    /// Apply a nolint-all action at the cursor position by running the actual linter.
    fn apply_nolint_all_at_cursor(source_with_cursor: &str) -> Option<String> {
        let cursor_pos = source_with_cursor.find(CURSOR)?;
        let content = source_with_cursor.replace(CURSOR, "");

        let env = TestEnv::new(&content);
        let snapshot = env.create_snapshot(&content);

        // Run the linter to get real diagnostics
        let diagnostics = lint::lint_document(&snapshot).ok()?;

        // Find the diagnostic at cursor position
        let cursor_lsp_pos = offset_to_position(&content, cursor_pos);
        let diagnostic = diagnostics
            .iter()
            .find(|d| position_in_range(cursor_lsp_pos, &d.range))?;

        // Get the nolint-all action
        let action = Server::diagnostic_to_nolint_all_action(diagnostic, &snapshot)?;
        let edit = action.edit?;
        let changes = edit.changes?;
        let text_edits = changes.values().next()?;

        // Apply edits
        let mut result = content.clone();
        for text_edit in text_edits.iter().rev() {
            let start = position_to_offset(&result, text_edit.range.start);
            let end = position_to_offset(&result, text_edit.range.end);
            result.replace_range(start..end, &text_edit.new_text);
        }

        Some(result)
    }

    // =========================================================================
    // Server creation test
    // =========================================================================

    #[test]
    fn test_server_creation() {
        let (connection, _io_threads) = Connection::memory();
        let worker_threads = NonZeroUsize::new(1).unwrap();

        let result = Server::new(worker_threads, connection);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Quick fix snapshot tests (using real linter)
    // =========================================================================

    #[test]
    fn test_fix_one_violation() {
        let result = apply_fix_at_cursor(
            r#"<CURS>any(is.na(x))
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        anyNA(x)
        "#);
    }

    #[test]
    fn test_one_of_multiple_violations() {
        let result = apply_fix_at_cursor(
            r#"x = 1
<CURS>x = 2
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        x = 1
        x <- 2
        "#);
    }

    #[test]
    fn test_fix_multiline_violation() {
        let result = apply_fix_at_cursor(
            r#"any(
            <CURS>duplicated(x)
            )"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        anyDuplicated(x) > 0
        "#);
    }

    #[test]
    fn test_fix_no_action_without_fix_data() {
        let snapshot = create_test_snapshot("class(x) == \"foo\"\n");

        let diagnostic = types::Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 16)),
            severity: Some(types::DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("jarl".to_string()),
            message: "Use inherits()".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let result = Server::diagnostic_to_code_action(&diagnostic, &snapshot);
        assert!(result.is_none());
    }

    // =========================================================================
    // Nolint rule snapshot tests (using real linter)
    // =========================================================================

    #[test]
    fn test_nolint_insert_new_comment() {
        let result = apply_nolint_at_cursor(
            r#"<CURS>x = 1
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        # nolint: assignment
        x = 1
        "#);
    }

    #[test]
    fn test_nolint_any_is_na() {
        let result = apply_nolint_at_cursor(
            r#"<CURS>any(is.na(x))
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        # nolint: any_is_na
        any(is.na(x))
        "#);
    }

    #[test]
    fn test_nolint_update_existing_comment() {
        let result = apply_nolint_at_cursor(
            r#"# nolint: foo
<CURS>x = 1
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        # nolint: foo, assignment
        x = 1
        "#);
    }

    #[test]
    fn test_nolint_with_indentation() {
        let result = apply_nolint_at_cursor(
            r#"f <- function() {
  <CURS>x = 1
}
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        f <- function() {
          # nolint: assignment
          x = 1
        }
        "#);
    }

    #[test]
    fn test_nolint_no_duplicate_rule() {
        let result = apply_nolint_at_cursor(
            r#"# nolint: assignment
<CURS>x = 1
"#,
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_nolint_no_add_to_generic() {
        let result = apply_nolint_at_cursor(
            r#"# nolint
<CURS>x = 1
"#,
        );

        assert!(result.is_none());
    }

    // =========================================================================
    // Nolint all snapshot tests (using real linter)
    // =========================================================================

    #[test]
    fn test_nolint_all_insert_new() {
        let result = apply_nolint_all_at_cursor(
            r#"<CURS>x = 1
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        # nolint
        x = 1
        "#);
    }

    #[test]
    fn test_nolint_all_replaces_specific() {
        let result = apply_nolint_all_at_cursor(
            r#"# nolint: foo, bar
<CURS>x = 1
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        # nolint
        x = 1
        "#);
    }

    // =========================================================================
    // Action properties tests (non-snapshot)
    // =========================================================================

    #[test]
    fn test_fix_action_properties() {
        let content = "x = 1\n";
        let env = TestEnv::new(content);
        let snapshot = env.create_snapshot(content);

        let diagnostics = lint::lint_document(&snapshot).unwrap();
        let diagnostic = diagnostics.first().unwrap();

        let action = Server::diagnostic_to_code_action(diagnostic, &snapshot).unwrap();

        assert!(action.title.starts_with("Fix:"));
        assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
        assert!(action.is_preferred.unwrap_or(false));
    }

    #[test]
    fn test_nolint_action_properties() {
        let content = "x = 1\n";
        let env = TestEnv::new(content);
        let snapshot = env.create_snapshot(content);

        let diagnostics = lint::lint_document(&snapshot).unwrap();
        let diagnostic = diagnostics.first().unwrap();

        let action = Server::diagnostic_to_nolint_rule_action(diagnostic, &snapshot).unwrap();

        assert!(action.title.contains("assignment"));
        assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
        assert!(!action.is_preferred.unwrap_or(true));
    }

    #[test]
    fn test_nolint_all_action_properties() {
        let content = "x = 1\n";
        let env = TestEnv::new(content);
        let snapshot = env.create_snapshot(content);

        let diagnostics = lint::lint_document(&snapshot).unwrap();
        let diagnostic = diagnostics.first().unwrap();

        let action = Server::diagnostic_to_nolint_all_action(diagnostic, &snapshot).unwrap();

        assert_eq!(action.title, "Ignore all violations on this node.");
        assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
    }

    // =========================================================================
    // Unicode tests (using real linter)
    // =========================================================================

    #[test]
    fn test_fix_unicode_accent() {
        let result = apply_fix_at_cursor(
            r#"<CURS>héllo = 1
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        héllo <- 1
        "#);
    }

    #[test]
    fn test_fix_unicode_emoji() {
        let result = apply_fix_at_cursor(
            r#"<CURS>rocket_var = 2
"#,
        )
        .unwrap();

        insta::assert_snapshot!(result, @r#"
        rocket_var <- 2
        "#);
    }

    #[test]
    fn test_unicode_position_utf8_vs_utf16() {
        let content = "x = 1";

        let snapshot_utf8 = create_test_snapshot_with_encoding(content, PositionEncoding::UTF8);
        let snapshot_utf16 = create_test_snapshot_with_encoding(content, PositionEncoding::UTF16);

        let fix = DiagnosticFix {
            content: "x <- 1".to_string(),
            start: 0,
            end: 5,
            is_safe: true,
            rule_name: "assignment".to_string(),
        };

        let diagnostic_utf8 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 2), Position::new(0, 3)),
            "Use <- for assignment".to_string(),
            fix.clone(),
        );

        let diagnostic_utf16 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 2), Position::new(0, 3)),
            "Use <- for assignment".to_string(),
            fix,
        );

        let action_utf8 = Server::diagnostic_to_code_action(&diagnostic_utf8, &snapshot_utf8);
        let action_utf16 = Server::diagnostic_to_code_action(&diagnostic_utf16, &snapshot_utf16);

        assert!(action_utf8.is_some());
        assert!(action_utf16.is_some());
    }

    // =========================================================================
    // Utility function tests
    // =========================================================================

    #[test]
    fn test_ranges_overlap() {
        let range1 = Range::new(Position::new(0, 0), Position::new(0, 5));
        let range2 = Range::new(Position::new(0, 3), Position::new(0, 8));
        let range3 = Range::new(Position::new(0, 6), Position::new(0, 10));
        let range4 = Range::new(Position::new(1, 0), Position::new(1, 5));

        assert!(ranges_overlap(&range1, &range2));
        assert!(ranges_overlap(&range2, &range1));
        assert!(!ranges_overlap(&range1, &range3));
        assert!(!ranges_overlap(&range3, &range1));
        assert!(!ranges_overlap(&range1, &range4));
        assert!(ranges_overlap(&range1, &range1));
    }
}
