//! Main LSP server implementation for Flir
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

                // Trigger linting for push diagnostics (real-time as you type)
                let supports_pull_diagnostics = session.supports_pull_diagnostics();

                if !supports_pull_diagnostics {
                    if let Some(snapshot) = session.take_snapshot(params.text_document.uri) {
                        task_sender.send(Task::LintDocument {
                            snapshot: Box::new(snapshot),
                            client: session.client().clone(),
                        })?;
                    }
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

                if !supports_pull_diagnostics {
                    if let Some(snapshot) = session.take_snapshot(params.text_document.uri) {
                        task_sender.send(Task::LintDocument {
                            snapshot: Box::new(snapshot),
                            client: session.client().clone(),
                        })?;
                    }
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
                if let Some(action) = Self::diagnostic_to_code_action(&diagnostic, snapshot) {
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
    use crate::lint::DiagnosticFix;
    use crate::session::DocumentSnapshot;
    use lsp_server::Connection;
    use lsp_types::{
        CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier, Url,
    };

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
            source: Some("flir".to_string()),
            message,
            related_information: None,
            tags: None,
            data: Some(fix_data),
        }
    }

    #[test]
    fn test_server_creation() {
        let (connection, _io_threads) = Connection::memory();
        let worker_threads = NonZeroUsize::new(1).unwrap();

        let result = Server::new(worker_threads, connection);
        assert!(result.is_ok());
    }

    #[test]
    fn test_diagnostic_to_code_action_with_assignment_fix() {
        let snapshot = create_test_snapshot("x = 1\n");

        let fix = DiagnosticFix {
            content: "x <- 1".to_string(),
            start: 0, // replace entire assignment
            end: 5,   // end of "x = 1"
            is_safe: true,
        };

        let diagnostic = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 2), Position::new(0, 3)),
            "Use <- for assignment".to_string(),
            fix,
        );

        let result = Server::diagnostic_to_code_action(&diagnostic, &snapshot);

        assert!(result.is_some());
        let action = result.unwrap();

        assert_eq!(action.title, "Fix: Use <- for assignment");
        assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
        assert!(action.is_preferred.unwrap_or(false));
        assert!(action.edit.is_some());

        let edit = action.edit.unwrap();
        assert!(edit.changes.is_some());

        let changes = edit.changes.unwrap();
        assert_eq!(changes.len(), 1);

        let text_edits = changes.values().next().unwrap();
        assert_eq!(text_edits.len(), 1);
        assert_eq!(text_edits[0].new_text, "x <- 1");
    }

    #[test]
    fn test_diagnostic_to_code_action_with_no_fix() {
        let snapshot = create_test_snapshot("class(x) == \"foo\"\n");

        let diagnostic = types::Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 16)),
            severity: Some(types::DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("flir".to_string()),
            message: "Use inherits() instead of class() == \"...\"".to_string(),
            related_information: None,
            tags: None,
            data: None, // No fix data available for this lint
        };

        let result = Server::diagnostic_to_code_action(&diagnostic, &snapshot);
        assert!(result.is_none());
    }

    #[test]
    fn test_diagnostic_to_code_action_with_empty_fix() {
        let snapshot = create_test_snapshot("y = 2\n");

        let fix = DiagnosticFix {
            content: "".to_string(),
            start: 0,
            end: 0, // No range to fix
            is_safe: true,
        };

        let diagnostic = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 0), Position::new(0, 0)),
            "Empty fix data".to_string(),
            fix,
        );

        let result = Server::diagnostic_to_code_action(&diagnostic, &snapshot);
        assert!(result.is_none());
    }

    #[test]
    fn test_assignment_operator_quick_fix() {
        let snapshot = create_test_snapshot("x = 1\n");

        let fix = DiagnosticFix {
            content: "x <- 1".to_string(),
            start: 0,
            end: 5, // "x = 1"
            is_safe: true,
        };

        let diagnostic = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 2), Position::new(0, 3)), // position of "="
            "Use <- for assignment".to_string(),
            fix,
        );

        let action = Server::diagnostic_to_code_action(&diagnostic, &snapshot);

        assert!(action.is_some());
        let action = action.unwrap();

        assert_eq!(action.title, "Fix: Use <- for assignment");
        assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
        assert!(action.is_preferred.unwrap_or(false));

        // Verify the edit
        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let text_edits = changes.values().next().unwrap();
        assert_eq!(text_edits[0].new_text, "x <- 1");
    }

    #[test]
    fn test_any_is_na_quick_fix() {
        let snapshot = create_test_snapshot("result <- any(is.na(data$column))\n");

        let fix = DiagnosticFix {
            content: "anyNA(data$column)".to_string(),
            start: 10, // start of "any(is.na(...))"
            end: 33,   // end of "any(is.na(data$column))"
            is_safe: true,
        };

        let diagnostic = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 10), Position::new(0, 33)),
            "Use anyNA() instead of any(is.na())".to_string(),
            fix,
        );

        let action = Server::diagnostic_to_code_action(&diagnostic, &snapshot);

        assert!(action.is_some());
        let action = action.unwrap();

        assert_eq!(action.title, "Fix: Use anyNA() instead of any(is.na())");
        assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
        assert!(action.is_preferred.unwrap_or(false));

        // Verify the edit replaces with anyNA
        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let text_edits = changes.values().next().unwrap();
        assert_eq!(text_edits[0].new_text, "anyNA(data$column)");
    }

    #[test]
    fn test_class_comparison_no_quick_fix() {
        let snapshot = create_test_snapshot("if (class(obj) == \"data.frame\") { }\n");

        // This lint should NOT have a quick fix, only a recommendation
        let diagnostic = types::Diagnostic {
            range: Range::new(Position::new(0, 4), Position::new(0, 27)),
            severity: Some(types::DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("flir".to_string()),
            message: "Use inherits() instead of class() == \"...\"".to_string(),
            related_information: None,
            tags: None,
            data: None, // No fix data - this lint doesn't provide automatic fixes
        };

        let action = Server::diagnostic_to_code_action(&diagnostic, &snapshot);

        // Should return None because there's no fix data
        assert!(action.is_none());
    }

    #[test]
    fn test_multiple_assignment_fixes_in_document() {
        let snapshot = create_test_snapshot("x = 1\ny = 2\nz = 3\n");

        // Test multiple assignment fixes in a single document
        let test_cases = vec![
            (0, 5, "x <- 1", Position::new(0, 2), Position::new(0, 3)),
            (6, 11, "y <- 2", Position::new(1, 2), Position::new(1, 3)),
            (12, 17, "z <- 3", Position::new(2, 2), Position::new(2, 3)),
        ];

        // Track all actions to verify they can coexist
        let mut actions = Vec::new();

        for (start, end, replacement, range_start, range_end) in test_cases {
            let fix = DiagnosticFix {
                content: replacement.to_string(),
                start,
                end,
                is_safe: true,
            };

            let range = Range::new(range_start, range_end);
            let diagnostic =
                create_test_diagnostic_with_fix(range, "Use <- for assignment".to_string(), fix);

            let action = Server::diagnostic_to_code_action(&diagnostic, &snapshot);
            assert!(action.is_some());

            let action = action.unwrap();
            assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));
            assert!(action.is_preferred.unwrap_or(false));

            // Verify the replacement text
            let edit = action.edit.as_ref().unwrap();
            let changes = edit.changes.as_ref().unwrap();
            let text_edits = changes.values().next().unwrap();
            assert_eq!(text_edits[0].new_text, replacement);

            actions.push(action);
        }

        // Verify we generated all expected fixes
        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn test_code_action_params_integration() {
        let snapshot = create_test_snapshot("x = 1\ny = 2\nany(is.na(data))\n");

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: snapshot.uri().clone() },
            range: Range::new(Position::new(0, 0), Position::new(2, 16)), // All lines
            context: CodeActionContext {
                diagnostics: vec![],
                only: Some(vec![types::CodeActionKind::QUICKFIX]),
                trigger_kind: None,
            },
            partial_result_params: Default::default(),
            work_done_progress_params: Default::default(),
        };

        // Verify the params structure for multiple potential fixes
        assert_eq!(params.text_document.uri, *snapshot.uri());
        assert_eq!(params.range.start.line, 0);
        assert_eq!(params.range.end.line, 2);
        assert_eq!(
            params.context.only,
            Some(vec![types::CodeActionKind::QUICKFIX])
        );
    }

    #[test]
    fn test_ranges_overlap() {
        let range1 = Range::new(Position::new(0, 0), Position::new(0, 5));
        let range2 = Range::new(Position::new(0, 3), Position::new(0, 8));
        let range3 = Range::new(Position::new(0, 6), Position::new(0, 10));
        let range4 = Range::new(Position::new(1, 0), Position::new(1, 5));

        // Overlapping ranges
        assert!(ranges_overlap(&range1, &range2));
        assert!(ranges_overlap(&range2, &range1));

        // Non-overlapping ranges
        assert!(!ranges_overlap(&range1, &range3));
        assert!(!ranges_overlap(&range3, &range1));

        // Different lines
        assert!(!ranges_overlap(&range1, &range4));
        assert!(!ranges_overlap(&range4, &range1));

        // Same range
        assert!(ranges_overlap(&range1, &range1));
    }

    #[test]
    fn test_unicode_diagnostics_and_fixes() {
        // Test that diagnostics and fixes work correctly with multibyte Unicode characters
        // Use simpler test cases to avoid byte boundary issues

        // Test case 1: Accent character
        let content1 = "héllo = 1";
        let snapshot1 = create_test_snapshot(content1);

        let fix1 = DiagnosticFix {
            content: "héllo <- 1".to_string(),
            start: 0,
            end: content1.len(),
            is_safe: true,
        };

        let diagnostic1 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 6), Position::new(0, 7)), // position of "="
            "Use <- for assignment".to_string(),
            fix1,
        );

        let action1 = Server::diagnostic_to_code_action(&diagnostic1, &snapshot1);
        assert!(
            action1.is_some(),
            "Failed to create action for accent character"
        );

        // Test case 2: Emoji
        let content2 = "🚀_var = 2";
        let snapshot2 = create_test_snapshot(content2);

        let fix2 = DiagnosticFix {
            content: "🚀_var <- 2".to_string(),
            start: 0,
            end: content2.len(),
            is_safe: true,
        };

        let diagnostic2 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 7), Position::new(0, 8)), // position of "="
            "Use <- for assignment".to_string(),
            fix2,
        );

        let action2 = Server::diagnostic_to_code_action(&diagnostic2, &snapshot2);
        assert!(action2.is_some(), "Failed to create action for emoji");

        // Test case 3: Chinese characters
        let content3 = "世界 = 3";
        let snapshot3 = create_test_snapshot(content3);

        let fix3 = DiagnosticFix {
            content: "世界 <- 3".to_string(),
            start: 0,
            end: content3.len(),
            is_safe: true,
        };

        let diagnostic3 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 3), Position::new(0, 4)), // position of "="
            "Use <- for assignment".to_string(),
            fix3,
        );

        let action3 = Server::diagnostic_to_code_action(&diagnostic3, &snapshot3);
        assert!(
            action3.is_some(),
            "Failed to create action for Chinese characters"
        );

        // Verify all actions have correct properties
        for (action, expected_text) in [
            (action1.unwrap(), "héllo <- 1"),
            (action2.unwrap(), "🚀_var <- 2"),
            (action3.unwrap(), "世界 <- 3"),
        ] {
            assert_eq!(action.title, "Fix: Use <- for assignment");
            assert_eq!(action.kind, Some(types::CodeActionKind::QUICKFIX));

            let edit = action.edit.unwrap();
            let changes = edit.changes.unwrap();
            let text_edits = changes.values().next().unwrap();
            assert_eq!(text_edits[0].new_text, expected_text);
        }
    }

    #[test]
    fn test_unicode_any_is_na_fix() {
        // Test anyNA fix with Unicode variable names
        let content = "résultat <- any(is.na(données$colonne))";
        let snapshot = create_test_snapshot(content);

        let fix = DiagnosticFix {
            content: "anyNA(données$colonne)".to_string(),
            start: 12, // start of "any(is.na(...))"
            end: 39,   // end of "any(is.na(données$colonne))"
            is_safe: true,
        };

        let diagnostic = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 12), Position::new(0, 39)),
            "Use anyNA() instead of any(is.na())".to_string(),
            fix,
        );

        let action = Server::diagnostic_to_code_action(&diagnostic, &snapshot);
        assert!(action.is_some());

        let action = action.unwrap();
        assert_eq!(action.title, "Fix: Use anyNA() instead of any(is.na())");

        // Verify Unicode is preserved in the fix
        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let text_edits = changes.values().next().unwrap();
        assert_eq!(text_edits[0].new_text, "anyNA(données$colonne)");
    }

    #[test]
    fn test_unicode_position_calculations() {
        // Test that position calculations work correctly with various Unicode scenarios
        use crate::document::PositionEncoding;

        let content = "🚀 = 1"; // Emoji takes 4 bytes in UTF-8, but 2 code units in UTF-16

        // Create snapshots with different encodings
        let snapshot_utf8 = create_test_snapshot_with_encoding(content, PositionEncoding::UTF8);
        let snapshot_utf16 = create_test_snapshot_with_encoding(content, PositionEncoding::UTF16);

        // Create a fix that targets the "=" character
        let fix = DiagnosticFix {
            content: "🚀 <- 1".to_string(),
            start: 5, // byte position of "=" in UTF-8
            end: 6,
            is_safe: true,
        };

        // Test UTF-8 encoding
        let diagnostic_utf8 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 2), Position::new(0, 3)), // character position of "="
            "Use <- for assignment".to_string(),
            fix.clone(),
        );

        let action_utf8 = Server::diagnostic_to_code_action(&diagnostic_utf8, &snapshot_utf8);
        assert!(
            action_utf8.is_some(),
            "UTF-8 encoding should work with emoji"
        );

        // Test UTF-16 encoding
        let diagnostic_utf16 = create_test_diagnostic_with_fix(
            Range::new(Position::new(0, 3), Position::new(0, 4)), // different char position in UTF-16
            "Use <- for assignment".to_string(),
            fix,
        );

        let action_utf16 = Server::diagnostic_to_code_action(&diagnostic_utf16, &snapshot_utf16);
        assert!(
            action_utf16.is_some(),
            "UTF-16 encoding should work with emoji"
        );

        // Both should produce the same replacement text
        let edit_utf8 = action_utf8.unwrap().edit.unwrap();
        let edit_utf16 = action_utf16.unwrap().edit.unwrap();

        let changes_utf8 = edit_utf8.changes.unwrap();
        let changes_utf16 = edit_utf16.changes.unwrap();

        let text_edit_utf8 = changes_utf8.values().next().unwrap();
        let text_edit_utf16 = changes_utf16.values().next().unwrap();

        assert_eq!(text_edit_utf8[0].new_text, "🚀 <- 1");
        assert_eq!(text_edit_utf16[0].new_text, "🚀 <- 1");
    }

    /// Helper function to create test snapshots with specific position encoding
    fn create_test_snapshot_with_encoding(
        content: &str,
        encoding: crate::document::PositionEncoding,
    ) -> DocumentSnapshot {
        let uri = lsp_types::Url::parse("file:///test_unicode.R").unwrap();
        let key = DocumentKey::from(uri);
        let document = TextDocument::new(content.to_string(), 1);

        DocumentSnapshot::new(
            document,
            key,
            encoding,
            lsp_types::ClientCapabilities::default(),
        )
    }
}
