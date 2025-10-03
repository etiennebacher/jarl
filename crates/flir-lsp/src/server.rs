//! Main LSP server implementation for Flir
//!
//! This module contains the core server logic that handles the LSP protocol,
//! focused purely on diagnostic (linting) capabilities. No code actions,
//! formatting, or other advanced features.

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
        snapshot: DocumentSnapshot,
        client: Client,
    },
    /// Handle a diagnostic request
    HandleDiagnosticRequest {
        snapshot: DocumentSnapshot,
        request_id: RequestId,
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
        eprintln!("FLIR LSP: Starting server (this should appear in VS Code logs)");
        eprintln!("FLIR LSP: Server.run() method called");
        tracing::info!("Starting LSP handshake");

        // Perform LSP handshake
        eprintln!("FLIR LSP: About to call initialize_start()");
        let (id, init_params) = self
            .connection
            .initialize_start()
            .context("Failed to start LSP initialization")?;
        eprintln!("FLIR LSP: initialize_start() completed successfully");
        eprintln!("FLIR LSP: Received initialize request with id: {id:?}");
        tracing::debug!("Received initialize request with id: {:?}", id);

        // Parse initialize params
        eprintln!("FLIR LSP: About to parse initialize params");
        let init_params: lsp_types::InitializeParams = serde_json::from_value(init_params)
            .context("Failed to parse initialization parameters")?;
        eprintln!("FLIR LSP: Parsed initialize params successfully");
        tracing::debug!("Parsed initialize params successfully");

        // Negotiate capabilities
        eprintln!("FLIR LSP: About to negotiate capabilities");
        let client_capabilities = init_params.capabilities.clone();
        let position_encoding = negotiate_position_encoding(&client_capabilities);

        tracing::info!("Negotiated position encoding: {:?}", position_encoding);
        eprintln!(
            "FLIR LSP: Position encoding negotiated: {position_encoding:?}"
        );

        // Create client for communication
        eprintln!("FLIR LSP: Creating client");
        let client = Client::new(self.connection.sender.clone());

        // Create session
        eprintln!("FLIR LSP: Creating session");
        let mut session = Session::new(
            client_capabilities,
            position_encoding,
            vec![], // Will be populated from init_params
            client.clone(),
        );

        // Initialize session and get initialize result
        eprintln!("FLIR LSP: About to initialize session");
        let initialize_result = session
            .initialize(init_params)
            .context("Failed to initialize session")?;
        eprintln!("FLIR LSP: Session initialized successfully");

        // Complete handshake
        eprintln!("FLIR LSP: About to serialize initialize result");
        eprintln!("FLIR LSP: Raw initialize result: {initialize_result:?}");
        let initialize_result_json = serde_json::to_value(initialize_result)
            .context("Failed to serialize initialize result")?;
        eprintln!(
            "FLIR LSP: Serialized initialize result JSON: {}",
            serde_json::to_string_pretty(&initialize_result_json)
                .unwrap_or_else(|_| "Failed to pretty print".to_string())
        );
        tracing::debug!("Initialize result: {:?}", initialize_result_json);

        eprintln!("FLIR LSP: About to call initialize_finish()");
        self.connection
            .initialize_finish(id, initialize_result_json)
            .context("Failed to finish LSP initialization")?;
        eprintln!("FLIR LSP: initialize_finish() completed successfully");

        eprintln!("FLIR LSP: Server initialized successfully");
        tracing::info!("LSP server initialized successfully");

        // Create worker thread pool
        eprintln!("FLIR LSP: Creating worker thread channels");
        let (task_sender, task_receiver) = channel::bounded::<Task>(100);
        let (event_sender, event_receiver) = channel::bounded::<Event>(100);

        // Spawn worker threads
        eprintln!(
            "FLIR LSP: Spawning {} worker threads",
            self.worker_threads.get()
        );
        let _worker_handles: Vec<_> = (0..self.worker_threads.get())
            .map(|i| {
                let task_receiver = task_receiver.clone();
                let event_sender = event_sender.clone();
                thread::spawn(move || {
                    tracing::debug!("Worker thread {} started", i);
                    Self::worker_thread(i, task_receiver, event_sender);
                    tracing::debug!("Worker thread {} stopped", i);
                })
            })
            .collect();

        // Run main loop
        eprintln!("FLIR LSP: About to start main loop");
        let result = self.main_loop(session, task_sender, event_receiver);
        eprintln!(
            "FLIR LSP: Main loop finished with result: {:?}",
            result.is_ok()
        );
        result
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
                            tracing::debug!("Received LSP message: {:?}", msg);
                            if let Err(e) = self.handle_message(msg, &mut session, &task_sender) {
                                eprintln!("FLIR LSP: Error handling message: {e}");
                                tracing::error!("Error handling message: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("FLIR LSP: Error receiving message: {e}");
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
                        snapshot,
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

                eprintln!("FLIR LSP: Opening document: {}", params.text_document.uri);
                tracing::info!("Opening document: {}", params.text_document.uri);

                let document =
                    TextDocument::new(params.text_document.text, params.text_document.version)
                        .with_language_id(&params.text_document.language_id);

                session.open_document(params.text_document.uri.clone(), document);

                // Trigger linting for push diagnostics (real-time as you type)
                if !session.supports_pull_diagnostics() {
                    if let Some(snapshot) = session.take_snapshot(params.text_document.uri) {
                        task_sender.send(Task::LintDocument {
                            snapshot,
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

                // Re-lint after document changes for push diagnostics
                if !session.supports_pull_diagnostics() {
                    if let Some(snapshot) = session.take_snapshot(params.text_document.uri) {
                        task_sender.send(Task::LintDocument {
                            snapshot,
                            client: session.client().clone(),
                        })?;
                    }
                }
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
                    if let Err(e) = Self::handle_lint_task(snapshot, client) {
                        tracing::error!("Error in lint task: {}", e);
                    }
                }
                Task::HandleDiagnosticRequest { snapshot, request_id, client } => {
                    if let Err(e) =
                        Self::handle_diagnostic_request(snapshot, request_id, client, &event_sender)
                    {
                        tracing::error!("Error in diagnostic request task: {}", e);
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_server::Connection;

    #[test]
    fn test_server_creation() {
        let (connection, _io_threads) = Connection::memory();
        let worker_threads = NonZeroUsize::new(1).unwrap();

        let result = Server::new(worker_threads, connection);
        assert!(result.is_ok());
    }
}
