//! Client communication for the Jarl LSP server
//!
//! This module handles sending messages to the LSP client, including notifications
//! and responses to requests.

use anyhow::Result;
use crossbeam::channel;
use gen_lsp_types::{self as types};
use lsp_server::{Message, Notification, Request, RequestId, Response, ResponseError};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

/// Client for sending messages to the LSP client
#[derive(Clone)]
pub struct Client {
    sender: channel::Sender<Message>,
    /// Counter for generating unique request IDs
    request_id_counter: Arc<std::sync::atomic::AtomicI32>,
    /// Pending outgoing requests waiting for responses
    pending_requests: Arc<std::sync::Mutex<HashMap<RequestId, PendingRequest>>>,
    /// Whether we've already shown the unused_function threshold notification
    /// this session. Shared across all clones so it fires at most once.
    unused_fn_threshold_notified: Arc<std::sync::atomic::AtomicBool>,
}

/// Information about a pending request sent to the client
#[derive(Debug)]
struct PendingRequest {
    method: String,
    sent_at: std::time::Instant,
}

impl Client {
    /// Create a new client with the given sender
    pub fn new(sender: channel::Sender<Message>) -> Self {
        Self {
            sender,
            request_id_counter: Arc::new(std::sync::atomic::AtomicI32::new(1)),
            pending_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
            unused_fn_threshold_notified: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Send a notification to the client
    pub fn send_notification<N: types::Notification>(&self, params: N::Params) -> Result<()>
    where
        N::Params: Serialize,
    {
        let notification = Notification {
            method: N::METHOD.to_string(),
            params: serde_json::to_value(params)?,
        };

        self.sender.send(Message::Notification(notification))?;

        Ok(())
    }

    /// Send a request to the client and register a response handler
    pub fn send_request<R: types::Request>(
        &self,
        params: R::Params,
        _handler: impl FnOnce(R::Result) + Send + 'static,
    ) -> Result<()>
    where
        R::Params: Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        let id = self.next_request_id();

        // Register the pending request
        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(
                id.clone(),
                PendingRequest {
                    method: R::METHOD.to_string(),
                    sent_at: std::time::Instant::now(),
                },
            );
        }

        let request = Request {
            id: id.clone(),
            method: R::METHOD.to_string(),
            params: serde_json::to_value(params)?,
        };

        self.sender.send(Message::Request(request))?;

        // In a real implementation, you'd store the handler and call it when
        // the response comes back. For this barebones version, we just log.
        tracing::debug!("Sent request {} with id {}", R::METHOD, id);

        Ok(())
    }

    /// Send a response to a client request
    pub fn send_response(&self, id: RequestId, result: impl Serialize) -> Result<()> {
        let response = Response {
            id,
            response_result: Ok(serde_json::to_value(result)?),
        };

        self.sender.send(Message::Response(response))?;
        Ok(())
    }

    /// Send an error response to a client request
    pub fn send_error_response(&self, id: RequestId, error: ResponseError) -> Result<()> {
        let response = Response { id, response_result: Err(error) };

        self.sender.send(Message::Response(response))?;
        Ok(())
    }

    /// Convenience method to publish diagnostics
    pub fn publish_diagnostics(
        &self,
        uri: types::Uri,
        diagnostics: Vec<types::Diagnostic>,
        version: Option<i32>,
    ) -> Result<()> {
        self.send_notification::<types::PublishDiagnosticsNotification>(
            types::PublishDiagnosticsParams { uri, diagnostics, version },
        )
    }

    /// Convenience method to show a message to the user
    pub fn show_message(&self, message: &str, message_type: types::MessageType) -> Result<()> {
        self.send_notification::<types::ShowMessageNotification>(types::ShowMessageParams {
            kind: message_type,
            message: message.to_string(),
        })
    }

    /// Show the unused_function threshold notification at most once per session.
    /// Returns `true` if the notification was actually sent (first call), `false`
    /// if it was already shown.
    pub fn notify_unused_fn_threshold_once(&self, hidden_count: usize) -> Result<bool> {
        let already = self
            .unused_fn_threshold_notified
            .swap(true, std::sync::atomic::Ordering::SeqCst);
        if already {
            return Ok(false);
        }
        let message = format!(
            "{hidden_count} `unused_function` diagnostic{s} hidden (likely false positives). \
             Adjust 'threshold-ignore' in `[lint.unused_function]` in jarl.toml to change this.\
             This message is shown once per session.",
            s = if hidden_count == 1 { "" } else { "s" },
        );
        self.show_message(&message, types::MessageType::Info)?;
        Ok(true)
    }

    /// Convenience method to log a message
    pub fn log_message(&self, message: &str, message_type: types::MessageType) -> Result<()> {
        self.send_notification::<types::LogMessageNotification>(types::LogMessageParams {
            kind: message_type,
            message: message.to_string(),
        })
    }

    /// Generate the next request ID
    fn next_request_id(&self) -> RequestId {
        let id = self
            .request_id_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        RequestId::from(id)
    }

    /// Handle a response from the client to one of our requests
    pub fn handle_response(&self, response: Response) {
        let mut pending = self.pending_requests.lock().unwrap();
        if let Some(pending_request) = pending.remove(&response.id) {
            let elapsed = pending_request.sent_at.elapsed();
            tracing::debug!(
                "Received response for {} request (id: {}) after {:?}",
                pending_request.method,
                response.id,
                elapsed
            );

            if let Err(error) = &response.response_result {
                error!(
                    "Request {} failed: {} - {}",
                    pending_request.method, error.code, error.message
                );
            }

            // In a full implementation, you would invoke the registered handler here
        } else {
            tracing::warn!("Received response for unknown request id: {}", response.id);
        }
    }

    /// Clean up old pending requests that never received a response
    pub fn cleanup_pending_requests(&self, timeout: std::time::Duration) {
        let mut pending = self.pending_requests.lock().unwrap();
        let now = std::time::Instant::now();
        let mut to_remove = Vec::new();

        for (id, request) in pending.iter() {
            if now.duration_since(request.sent_at) > timeout {
                tracing::warn!(
                    "Request {} (id: {}) timed out after {:?}",
                    request.method,
                    id,
                    now.duration_since(request.sent_at)
                );
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            pending.remove(&id);
        }
    }
}

/// Extension trait for converting errors to LSP ResponseError
pub trait ToLspError {
    fn to_lsp_error(self) -> ResponseError;
    fn to_lsp_error_with_code(self, code: i32) -> ResponseError;
}

impl ToLspError for anyhow::Error {
    fn to_lsp_error(self) -> ResponseError {
        ResponseError {
            code: lsp_server::ErrorCode::InternalError as i32,
            message: self.to_string(),
            data: None,
        }
    }

    fn to_lsp_error_with_code(self, code: i32) -> ResponseError {
        ResponseError { code, message: self.to_string(), data: None }
    }
}

/// Common LSP error codes
#[allow(dead_code)]
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // LSP-specific error codes
    pub const SERVER_CANCELLED: i32 = -32802;
    pub const CONTENT_MODIFIED: i32 = -32801;
    pub const REQUEST_CANCELLED: i32 = -32800;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> (Client, channel::Receiver<Message>) {
        let (sender, receiver) = channel::unbounded();
        (Client::new(sender), receiver)
    }

    /// Method and params of the next message the client sent
    fn next_notification(receiver: &channel::Receiver<Message>) -> (String, serde_json::Value) {
        match receiver.try_recv().expect("no message was sent") {
            Message::Notification(notification) => (notification.method, notification.params),
            other => panic!("expected a notification, got {other:?}"),
        }
    }

    fn test_diagnostic() -> types::Diagnostic {
        types::Diagnostic {
            range: types::Range::new(types::Position::new(0, 0), types::Position::new(0, 3)),
            severity: Some(types::DiagnosticSeverity::Warning),
            code: Some(types::Code::String("any_duplicated".to_string())),
            code_description: None,
            source: Some(crate::DIAGNOSTIC_SOURCE.to_string()),
            message: types::Message::String("Use anyDuplicated()".to_string()),
            tags: None,
            related_information: None,
            data: None,
        }
    }

    #[test]
    fn test_client_creation() {
        let (client, _receiver) = create_test_client();
        // Just test that we can create a client
        assert_eq!(client.next_request_id(), RequestId::from(1));
        assert_eq!(client.next_request_id(), RequestId::from(2));
    }

    #[test]
    fn test_error_conversion() {
        let error = anyhow::anyhow!("Test error");
        let lsp_error = error.to_lsp_error();
        assert_eq!(lsp_error.code, lsp_server::ErrorCode::InternalError as i32);
        assert_eq!(lsp_error.message, "Test error");
    }

    #[test]
    fn test_send_request_uses_protocol_method_and_tracks_it() {
        let (client, receiver) = create_test_client();

        client
            .send_request::<types::WorkspaceFoldersRequest>((), |_| {})
            .unwrap();

        let request = match receiver.try_recv().expect("no message was sent") {
            Message::Request(request) => request,
            other => panic!("expected a request, got {other:?}"),
        };
        assert_eq!(request.method, "workspace/workspaceFolders");
        assert_eq!(request.id, RequestId::from(1));
        assert_eq!(client.pending_requests.lock().unwrap().len(), 1);

        // Receiving the response retires the pending entry.
        client.handle_response(Response::new_ok(
            RequestId::from(1),
            serde_json::Value::Null,
        ));
        assert!(client.pending_requests.lock().unwrap().is_empty());
    }

    #[test]
    fn test_send_response_wire_format() {
        let (client, receiver) = create_test_client();

        client
            .send_response(RequestId::from(1), serde_json::json!({ "ok": true }))
            .unwrap();
        client
            .send_error_response(RequestId::from(2), anyhow::anyhow!("boom").to_lsp_error())
            .unwrap();

        let success = match receiver.try_recv().expect("no message was sent") {
            Message::Response(response) => serde_json::to_value(response).unwrap(),
            other => panic!("expected a response, got {other:?}"),
        };
        assert_eq!(success["id"], 1);
        assert_eq!(success["result"]["ok"], true);

        let failure = match receiver.try_recv().expect("no message was sent") {
            Message::Response(response) => serde_json::to_value(response).unwrap(),
            other => panic!("expected a response, got {other:?}"),
        };
        assert_eq!(failure["id"], 2);
        assert_eq!(failure["error"]["code"], error_codes::INTERNAL_ERROR);
        assert_eq!(failure["error"]["message"], "boom");
    }

    #[test]
    fn test_cleanup_pending_requests_drops_timed_out_entries() {
        let (client, _receiver) = create_test_client();

        client
            .send_request::<types::WorkspaceFoldersRequest>((), |_| {})
            .unwrap();
        assert_eq!(client.pending_requests.lock().unwrap().len(), 1);

        client.cleanup_pending_requests(std::time::Duration::from_secs(60));
        assert_eq!(client.pending_requests.lock().unwrap().len(), 1);

        std::thread::sleep(std::time::Duration::from_millis(1));
        client.cleanup_pending_requests(std::time::Duration::ZERO);
        assert!(client.pending_requests.lock().unwrap().is_empty());
    }

    #[test]
    fn test_publish_diagnostics_wire_format() {
        let (client, receiver) = create_test_client();
        let uri = types::Uri::parse("file:///test.R").unwrap();

        client
            .publish_diagnostics(uri, vec![test_diagnostic()], Some(7))
            .unwrap();

        let (method, params) = next_notification(&receiver);
        assert_eq!(method, "textDocument/publishDiagnostics");
        assert_eq!(params["uri"], "file:///test.R");
        assert_eq!(params["version"], 7);

        let diagnostic = &params["diagnostics"][0];
        assert_eq!(diagnostic["severity"], 2);
        assert_eq!(diagnostic["source"], "Jarl");
        assert_eq!(diagnostic["range"]["end"]["character"], 3);
        // `code` and `message` are untagged enums, so they have to reach the
        // client as bare values rather than as tagged objects.
        assert_eq!(diagnostic["code"], "any_duplicated");
        assert_eq!(diagnostic["message"], "Use anyDuplicated()");
    }

    #[test]
    fn test_show_message_wire_format() {
        let (client, receiver) = create_test_client();

        client
            .show_message("hello", types::MessageType::Info)
            .unwrap();

        let (method, params) = next_notification(&receiver);
        assert_eq!(method, "window/showMessage");
        // The field is `kind` in Rust but the protocol calls it `type`.
        assert_eq!(params["type"], 3);
        assert_eq!(params["message"], "hello");
    }

    #[test]
    fn test_log_message_wire_format() {
        let (client, receiver) = create_test_client();

        client
            .log_message("logged", types::MessageType::Log)
            .unwrap();

        let (method, params) = next_notification(&receiver);
        assert_eq!(method, "window/logMessage");
        assert_eq!(params["type"], 4);
        assert_eq!(params["message"], "logged");
    }

    #[test]
    fn test_unused_fn_threshold_notified_once() {
        let (client, receiver) = create_test_client();

        assert!(client.notify_unused_fn_threshold_once(3).unwrap());
        assert!(!client.notify_unused_fn_threshold_once(3).unwrap());

        let (method, params) = next_notification(&receiver);
        assert_eq!(method, "window/showMessage");
        assert_eq!(params["type"], 3);
        assert!(
            receiver.try_recv().is_err(),
            "the notification should only be sent once per session"
        );
    }
}
