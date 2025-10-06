//! Session management for the Flir LSP server
//!
//! This module handles the overall state of the LSP session, including
//! document management, client capabilities, and workspace configuration.

use anyhow::{Result, anyhow};
use lsp_types::{
    ClientCapabilities, CodeActionKind, CodeActionOptions, CodeActionProviderCapability,
    InitializeParams, InitializeResult, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Url,
    WorkDoneProgressOptions,
};
use rustc_hash::FxHashMap;

use std::path::PathBuf;

use crate::LspResult;
use crate::client::Client;
use crate::document::{DocumentKey, DocumentVersion, PositionEncoding, TextDocument};

/// Main session state for the LSP server
pub struct Session {
    /// Documents currently open in the editor
    documents: FxHashMap<DocumentKey, TextDocument>,
    /// Client capabilities negotiated during initialization
    client_capabilities: ClientCapabilities,
    /// Position encoding negotiated with the client
    position_encoding: PositionEncoding,
    /// Whether the client has requested shutdown
    shutdown_requested: bool,
    /// Workspace root paths
    workspace_roots: Vec<PathBuf>,
    /// Client for sending messages
    client: Client,
}

/// Immutable snapshot of a document and its context
pub struct DocumentSnapshot {
    /// The document content and metadata
    document: TextDocument,
    /// The document key
    key: DocumentKey,
    /// Position encoding for this session
    position_encoding: PositionEncoding,
    /// Client capabilities
    client_capabilities: ClientCapabilities,
}

impl Session {
    /// Create a new session with the given client capabilities
    pub fn new(
        client_capabilities: ClientCapabilities,
        position_encoding: PositionEncoding,
        workspace_roots: Vec<PathBuf>,
        client: Client,
    ) -> Self {
        Self {
            documents: FxHashMap::default(),
            client_capabilities,
            position_encoding,
            shutdown_requested: false,
            workspace_roots,
            client,
        }
    }

    /// Initialize the session with client parameters
    #[allow(deprecated)]
    pub fn initialize(&mut self, params: InitializeParams) -> LspResult<InitializeResult> {
        // Update workspace roots if provided
        if let Some(workspace_folders) = params.workspace_folders {
            self.workspace_roots.clear();
            for folder in workspace_folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    self.workspace_roots.push(path);
                }
            }
        } else if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                self.workspace_roots = vec![path];
            }
        } else if let Some(root_path) = params.root_path {
            self.workspace_roots = vec![PathBuf::from(root_path)];
        }

        tracing::info!(
            "Initialized Flir LSP with {} workspace roots (diagnostics only)",
            self.workspace_roots.len()
        );

        Ok(InitializeResult {
            capabilities: self.server_capabilities(),
            server_info: Some(ServerInfo {
                name: "Flir Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    /// Get the server capabilities that we support
    pub fn server_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            position_encoding: Some(self.position_encoding.into()),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    will_save: Some(false),
                    will_save_wait_until: Some(false),
                    save: Some(SaveOptions { include_text: Some(false) }.into()),
                },
            )),
            diagnostic_provider: None, // Use push diagnostics only
            // Add code action support for quick fixes
            hover_provider: None,
            completion_provider: None,
            code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            workspace: None,
            ..Default::default()
        }
    }

    /// Open a new text document
    pub fn open_document(&mut self, uri: Url, document: TextDocument) {
        let key = DocumentKey::from(uri);
        tracing::debug!("Opening document: {}", key.uri());
        self.documents.insert(key, document);
    }

    /// Update an existing document with changes
    pub fn update_document(
        &mut self,
        uri: Url,
        changes: Vec<lsp_types::TextDocumentContentChangeEvent>,
        version: DocumentVersion,
    ) -> LspResult<()> {
        let key = DocumentKey::from(uri);

        eprintln!(
            "FLIR LSP: Updating document {} with {} changes to version {}",
            key.uri(),
            changes.len(),
            version
        );

        let document = self
            .documents
            .get_mut(&key)
            .ok_or_else(|| anyhow!("Document not found: {}", key.uri()))?;

        document.apply_changes(changes, version, self.position_encoding)?;

        tracing::debug!("Updated document: {} to version {}", key.uri(), version);
        Ok(())
    }

    /// Close a document
    pub fn close_document(&mut self, uri: Url) -> LspResult<()> {
        let key = DocumentKey::from(uri);

        if self.documents.remove(&key).is_some() {
            tracing::debug!("Closed document: {}", key.uri());
            Ok(())
        } else {
            Err(anyhow!("Document not found: {}", key.uri()))
        }
    }

    /// Get a document by URI
    pub fn get_document(&self, uri: &Url) -> Option<&TextDocument> {
        let key = DocumentKey::from(uri.clone());
        self.documents.get(&key)
    }

    /// Take a snapshot of a document
    pub fn take_snapshot(&self, uri: Url) -> Option<DocumentSnapshot> {
        let key = DocumentKey::from(uri);
        let document = self.documents.get(&key)?;

        Some(DocumentSnapshot {
            document: document.clone(),
            key,
            position_encoding: self.position_encoding,
            client_capabilities: self.client_capabilities.clone(),
        })
    }

    /// Get all open document URIs
    pub fn open_documents(&self) -> impl Iterator<Item = &Url> {
        self.documents.keys().map(|key| key.uri())
    }

    /// Check if the client supports pull diagnostics
    /// For FLIR, we always prefer push diagnostics for real-time linting
    pub fn supports_pull_diagnostics(&self) -> bool {
        // Always use push diagnostics for immediate feedback
        // This ensures diagnostics are sent automatically on document changes
        false
    }

    /// Get the position encoding
    pub fn position_encoding(&self) -> PositionEncoding {
        self.position_encoding
    }

    /// Get the client capabilities
    pub fn client_capabilities(&self) -> &ClientCapabilities {
        &self.client_capabilities
    }

    /// Get the workspace roots
    pub fn workspace_roots(&self) -> &[PathBuf] {
        &self.workspace_roots
    }

    /// Mark that shutdown has been requested
    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
        tracing::info!("Shutdown requested");
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }

    /// Get the client for sending messages
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the number of open documents
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
}

impl DocumentSnapshot {
    /// Create a new document snapshot
    pub fn new(
        document: TextDocument,
        key: DocumentKey,
        position_encoding: PositionEncoding,
        client_capabilities: ClientCapabilities,
    ) -> Self {
        Self {
            document,
            key,
            position_encoding,
            client_capabilities,
        }
    }

    /// Get the document content
    pub fn content(&self) -> &str {
        self.document.content()
    }

    /// Get the document version
    pub fn version(&self) -> DocumentVersion {
        self.document.version()
    }

    /// Get the document key
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    /// Get the document URI
    pub fn uri(&self) -> &Url {
        self.key.uri()
    }

    /// Get the file path if this is a file URI
    pub fn file_path(&self) -> Option<PathBuf> {
        self.key.file_path()
    }

    /// Get the position encoding
    pub fn position_encoding(&self) -> PositionEncoding {
        self.position_encoding
    }

    /// Get the client capabilities
    pub fn client_capabilities(&self) -> &ClientCapabilities {
        &self.client_capabilities
    }

    /// Get the language ID if available
    pub fn language_id(&self) -> Option<&str> {
        self.document.language_id()
    }

    /// Convert a position to byte offset
    pub fn position_to_offset(&self, position: lsp_types::Position) -> Result<usize> {
        self.document
            .position_to_offset(position, self.position_encoding)
    }

    /// Convert a byte offset to position
    pub fn offset_to_position(&self, offset: usize) -> Result<lsp_types::Position> {
        self.document
            .offset_to_position(offset, self.position_encoding)
    }

    /// Get a range as a Range
    pub fn range_of_span(&self, start: usize, end: usize) -> Result<lsp_types::Range> {
        self.document
            .range_of_text(start, end, self.position_encoding)
    }
}

/// Determine the best position encoding from client capabilities
pub fn negotiate_position_encoding(client_capabilities: &ClientCapabilities) -> PositionEncoding {
    let supported_encodings = client_capabilities
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref());

    if let Some(encodings) = supported_encodings {
        // Prefer UTF-8 if supported, then UTF-16 (LSP default), then UTF-32
        for encoding in encodings {
            if let Ok(pos_encoding) = PositionEncoding::try_from(encoding) {
                match pos_encoding {
                    PositionEncoding::UTF8 => return PositionEncoding::UTF8,
                    _ => continue,
                }
            }
        }

        // Check for UTF-16 (LSP default)
        for encoding in encodings {
            if let Ok(pos_encoding) = PositionEncoding::try_from(encoding) {
                match pos_encoding {
                    PositionEncoding::UTF16 => return PositionEncoding::UTF16,
                    _ => continue,
                }
            }
        }
    }

    // Default to UTF-16 as per LSP specification
    PositionEncoding::UTF16
}

#[cfg(test)]
mod tests {
    use super::*;

    use lsp_types::{ClientCapabilities, GeneralClientCapabilities, PositionEncodingKind};

    fn create_test_session() -> Session {
        let (sender, _receiver) = crossbeam::channel::unbounded();
        let client = Client::new(sender);
        Session::new(
            ClientCapabilities::default(),
            PositionEncoding::UTF16,
            vec![],
            client,
        )
    }

    #[test]
    fn test_session_creation() {
        let session = create_test_session();
        assert_eq!(session.document_count(), 0);
        assert!(!session.is_shutdown_requested());
    }

    #[test]
    fn test_document_lifecycle() {
        let mut session = create_test_session();
        let uri = Url::parse("file:///test.py").unwrap();
        let document = TextDocument::new("hello world".to_string(), 1);

        // Open document
        session.open_document(uri.clone(), document);
        assert_eq!(session.document_count(), 1);
        assert!(session.get_document(&uri).is_some());

        // Take snapshot
        let snapshot = session.take_snapshot(uri.clone());
        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.content(), "hello world");
        assert_eq!(snapshot.version(), 1);

        // Close document
        session.close_document(uri.clone()).unwrap();
        assert_eq!(session.document_count(), 0);
        assert!(session.get_document(&uri).is_none());
    }

    #[test]
    fn test_position_encoding_negotiation() {
        // Test UTF-8 preference
        let mut caps = ClientCapabilities::default();
        caps.general = Some(GeneralClientCapabilities {
            position_encodings: Some(vec![
                PositionEncodingKind::UTF8,
                PositionEncodingKind::UTF16,
            ]),
            ..Default::default()
        });

        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::UTF8);

        // Test UTF-16 fallback
        caps.general = Some(GeneralClientCapabilities {
            position_encodings: Some(vec![PositionEncodingKind::UTF16]),
            ..Default::default()
        });

        assert_eq!(negotiate_position_encoding(&caps), PositionEncoding::UTF16);

        // Test default when no encodings specified
        let default_caps = ClientCapabilities::default();
        assert_eq!(
            negotiate_position_encoding(&default_caps),
            PositionEncoding::UTF16
        );
    }

    #[test]
    fn test_server_capabilities() {
        let session = create_test_session();
        let caps = session.server_capabilities();

        assert!(caps.text_document_sync.is_some());
        assert!(caps.diagnostic_provider.is_none());

        if let Some(sync) = caps.text_document_sync {
            if let TextDocumentSyncCapability::Options(options) = sync {
                assert_eq!(options.open_close, Some(true));
                assert_eq!(options.change, Some(TextDocumentSyncKind::INCREMENTAL));
            }
        }
    }
}
