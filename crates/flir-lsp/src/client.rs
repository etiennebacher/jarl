//! Client communication for the Flir LSP server
//!
//! This module handles sending messages to the LSP client, including notifications
//! and responses to requests.

use anyhow::Result;
use crossbeam::channel;
use lsp_server::{Message, Notification, Request, RequestId, Response, ResponseError};
use lsp_types::{self as types};
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
        }
    }

    /// Send a notification to the client
    pub fn send_notification<N: types::notification::Notification>(
        &self,
        params: N::Params,
    ) -> Result<()>
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
    pub fn send_request<R: types::request::Request>(
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
            result: Some(serde_json::to_value(result)?),
            error: None,
        };

        self.sender.send(Message::Response(response))?;
        Ok(())
    }

    /// Send an error response to a client request
    pub fn send_error_response(&self, id: RequestId, error: ResponseError) -> Result<()> {
        let response = Response { id, result: None, error: Some(error) };

        self.sender.send(Message::Response(response))?;
        Ok(())
    }

    /// Convenience method to publish diagnostics
    pub fn publish_diagnostics(
        &self,
        uri: types::Url,
        diagnostics: Vec<types::Diagnostic>,
        version: Option<i32>,
    ) -> Result<()> {
        self.send_notification::<types::notification::PublishDiagnostics>(
            types::PublishDiagnosticsParams { uri, diagnostics, version },
        )
    }

    /// Convenience method to show a message to the user
    pub fn show_message(&self, message: &str, message_type: types::MessageType) -> Result<()> {
        self.send_notification::<types::notification::ShowMessage>(types::ShowMessageParams {
            typ: message_type,
            message: message.to_string(),
        })
    }

    /// Convenience method to log a message
    pub fn log_message(&self, message: &str, message_type: types::MessageType) -> Result<()> {
        self.send_notification::<types::notification::LogMessage>(types::LogMessageParams {
            typ: message_type,
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

            if let Some(error) = &response.error {
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
    use std::sync::mpsc;

    fn create_test_client() -> (Client, mpsc::Receiver<Message>) {
        let (sender, _receiver) = channel::unbounded();
        let client = Client::new(sender);
        // Convert crossbeam receiver to mpsc for compatibility
        let (_mpsc_sender, mpsc_receiver) = mpsc::channel();
        (client, mpsc_receiver)
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
}
