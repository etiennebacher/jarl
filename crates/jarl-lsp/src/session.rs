//! Session management for the Jarl LSP server
//!
//! This module handles the overall state of the LSP session, including
//! document management, client capabilities, and workspace configuration.

use anyhow::{Result, anyhow};
use gen_lsp_types::{
    ClientCapabilities, CodeActionKind, CodeActionOptions, CodeActionProvider, InitializeParams,
    InitializeResult, MessageType, Position, Range, RootPath, SaveOptions, ServerCapabilities,
    ServerInfo, TextDocumentContentChangeEvent, TextDocumentSync, TextDocumentSyncKind,
    TextDocumentSyncOptions, Uri, WorkDoneProgressOptions, WorkspaceFolders,
};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use std::path::PathBuf;
use std::sync::Arc;

use jarl_core::package_cache::PackageCacheMap;

use crate::LspResult;
use crate::client::Client;
use crate::document::{DocumentKey, DocumentVersion, PositionEncoding, TextDocument};

/// Initialization options sent by the client
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitializationOptions {
    /// Log level for the server
    pub log_level: Option<String>,
    /// Log levels for dependencies
    pub dependency_log_levels: Option<String>,
}

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
    /// Whether we've shown the config notification
    config_notification_shown: bool,
    /// Per-project package caches for package-specific rules. Keyed by R
    /// project root so that renv and system projects get separate caches.
    package_cache_map: Arc<PackageCacheMap>,
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
    /// Shared reference to the session-level cache map. The lint code
    /// creates per-project caches on first use.
    package_cache_map: Arc<PackageCacheMap>,
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
            config_notification_shown: false,
            package_cache_map: Arc::new(PackageCacheMap::new()),
        }
    }

    /// Initialize the session with client parameters
    #[allow(deprecated)]
    pub fn initialize(&mut self, params: InitializeParams) -> LspResult<InitializeResult> {
        // Update workspace roots if provided
        if let Some(WorkspaceFolders::WorkspaceFolderList(workspace_folders)) =
            params.workspace_folders_initialize_params.workspace_folders
        {
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
        } else if let Some(RootPath::String(root_path)) = params.root_path {
            self.workspace_roots = vec![PathBuf::from(root_path)];
        }

        tracing::info!(
            "Initialized Jarl LSP with {} workspace roots (diagnostics only)",
            self.workspace_roots.len()
        );

        Ok(InitializeResult {
            capabilities: self.server_capabilities(),
            server_info: Some(ServerInfo {
                name: "Jarl Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    /// Get the server capabilities that we support
    pub fn server_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            position_encoding: Some(self.position_encoding.into()),
            text_document_sync: Some(TextDocumentSync::Options(TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::Incremental),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: Some(SaveOptions { include_text: Some(false) }.into()),
            })),
            diagnostic_provider: None, // Use push diagnostics only
            // Add code action support for quick fixes
            hover_provider: None,
            completion_provider: None,
            code_action_provider: Some(CodeActionProvider::CodeActionOptions(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QuickFix]),
                documentation: None,
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            workspace: None,
            ..Default::default()
        }
    }

    /// Open a new text document
    pub fn open_document(&mut self, uri: Uri, document: TextDocument) {
        let key = DocumentKey::from(uri);
        tracing::debug!("Opening document: {}", key.uri());
        self.documents.insert(key, document);
    }

    /// Update an existing document with changes
    pub fn update_document(
        &mut self,
        uri: Uri,
        changes: Vec<TextDocumentContentChangeEvent>,
        version: DocumentVersion,
    ) -> LspResult<()> {
        let key = DocumentKey::from(uri);

        eprintln!(
            "JARL LSP: Updating document {} with {} changes to version {}",
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
    pub fn close_document(&mut self, uri: Uri) -> LspResult<()> {
        let key = DocumentKey::from(uri);

        if self.documents.remove(&key).is_some() {
            tracing::debug!("Closed document: {}", key.uri());
            Ok(())
        } else {
            Err(anyhow!("Document not found: {}", key.uri()))
        }
    }

    /// Get a document by URI
    pub fn get_document(&self, uri: &Uri) -> Option<&TextDocument> {
        let key = DocumentKey::from(uri.clone());
        self.documents.get(&key)
    }

    /// Take a snapshot of a document
    pub fn take_snapshot(&self, uri: Uri) -> Option<DocumentSnapshot> {
        let key = DocumentKey::from(uri);
        let document = self.documents.get(&key)?;

        Some(DocumentSnapshot {
            document: document.clone(),
            key,
            position_encoding: self.position_encoding,
            client_capabilities: self.client_capabilities.clone(),
            package_cache_map: Arc::clone(&self.package_cache_map),
        })
    }

    /// Get the shared cache map.
    pub fn package_cache_map(&self) -> &Arc<PackageCacheMap> {
        &self.package_cache_map
    }

    /// Get all open document URIs
    pub fn open_documents(&self) -> impl Iterator<Item = &Uri> {
        self.documents.keys().map(|key| key.uri())
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

    /// Check and notify about config file location if needed
    /// Returns true if notification was shown, false otherwise
    pub fn check_and_notify_config(&mut self, file_path: &std::path::Path) -> bool {
        let cwd = match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(_) => return false,
        };
        self.check_and_notify_config_with_cwd(file_path, &cwd)
    }

    /// Check and notify about config file location if needed, using an
    /// explicit `cwd` instead of reading `env::current_dir()`.
    fn check_and_notify_config_with_cwd(
        &mut self,
        file_path: &std::path::Path,
        cwd: &std::path::Path,
    ) -> bool {
        use jarl_core::discovery::discover_settings;

        // Only show notification once per session
        if self.config_notification_shown {
            return false;
        }

        // Canonicalize CWD to handle symlinks (especially on macOS where
        // /tmp -> /private/tmp)
        let cwd_canonical = match cwd.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Discover settings for this file
        let file_path_str = vec![file_path.to_string_lossy().to_string()];
        let discovered_settings = match discover_settings(&file_path_str) {
            Ok(settings) => settings,
            Err(_) => return false,
        };

        // Check if any config is from a parent directory (not CWD)
        for ds in discovered_settings {
            if let Some(config_path) = &ds.config_path
                && let Some(config_dir) = config_path.parent()
            {
                // Canonicalize config_dir to handle symlinks
                let config_dir_canonical = match config_dir.canonicalize() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                if config_dir_canonical != cwd_canonical {
                    // Config is from a parent directory, show notification
                    if let Err(e) = self.client.show_message(
                        &format!(
                            "Jarl uses the configuration from '{}'",
                            config_path.display()
                        ),
                        MessageType::Info,
                    ) {
                        tracing::error!("Failed to show config notification: {}", e);
                    } else {
                        tracing::info!("Showed config notification for: {}", config_path.display());
                    }
                    self.config_notification_shown = true;
                    return true;
                }
            }
        }

        false
    }
}

impl DocumentSnapshot {
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
            package_cache_map: Arc::new(PackageCacheMap::new()),
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
    pub fn uri(&self) -> &Uri {
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

    /// Get or create the package cache for this document's project root.
    pub fn get_or_create_package_cache(
        &self,
        packages: &[&str],
    ) -> Option<Arc<jarl_core::package_cache::PackageCache>> {
        let file_path = self.file_path()?;
        self.package_cache_map.get_or_create(&file_path, packages)
    }

    /// Get the existing package cache for this document's project root, if any.
    pub fn package_cache(&self) -> Option<Arc<jarl_core::package_cache::PackageCache>> {
        let file_path = self.file_path()?;
        self.package_cache_map.get_for_file(&file_path)
    }

    /// Get the language ID if available
    pub fn language_id(&self) -> Option<&str> {
        self.document.language_id()
    }

    /// Convert a position to byte offset
    pub fn position_to_offset(&self, position: Position) -> Result<usize> {
        self.document
            .position_to_offset(position, self.position_encoding)
    }

    /// Convert a byte offset to position
    pub fn offset_to_position(&self, offset: usize) -> Result<Position> {
        self.document
            .offset_to_position(offset, self.position_encoding)
    }

    /// Get a range as a Range
    pub fn range_of_span(&self, start: usize, end: usize) -> Result<Range> {
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

    use gen_lsp_types::{GeneralClientCapabilities, PositionEncodingKind};

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

    /// Initialize a session from a raw `initialize` payload and return the
    /// workspace roots it derived.
    fn workspace_roots_from(params_json: serde_json::Value) -> Vec<PathBuf> {
        let params: InitializeParams = serde_json::from_value(params_json).unwrap();
        let mut session = create_test_session();
        session.initialize(params).unwrap();
        session.workspace_roots.clone()
    }

    /// The `file://` URI for an absolute path. Built from a real path rather
    /// than hardcoded, because a URI like `file:///project/a` carries no drive
    /// letter and `to_file_path` rejects it on Windows.
    fn file_uri(path: &std::path::Path) -> String {
        Uri::from_file_path(path).unwrap().to_string()
    }

    #[test]
    fn test_initialize_reads_workspace_folders() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let first = temp_dir.path().join("a");
        let second = temp_dir.path().join("b");

        // `workspaceFolders` is flattened into a nested params struct, so it
        // still has to be picked up from the top level of the payload.
        let roots = workspace_roots_from(serde_json::json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {},
            "workspaceFolders": [
                { "uri": file_uri(&first), "name": "a" },
                { "uri": file_uri(&second), "name": "b" }
            ]
        }));

        assert_eq!(roots, vec![first, second]);
    }

    #[test]
    fn test_initialize_falls_back_to_root_uri() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path().join("c");

        // A client that supports workspace folders but has none configured sends
        // an explicit `null`, which is a distinct value from the absent field —
        // both fall through to `rootUri`.
        let with_explicit_null = workspace_roots_from(serde_json::json!({
            "processId": null,
            "rootUri": file_uri(&root),
            "capabilities": {},
            "workspaceFolders": null,
        }));
        assert_eq!(with_explicit_null, vec![root.clone()]);

        let with_absent_field = workspace_roots_from(serde_json::json!({
            "processId": null,
            "rootUri": file_uri(&root),
            "capabilities": {},
        }));
        assert_eq!(with_absent_field, vec![root]);
    }

    #[test]
    fn test_initialize_falls_back_to_root_path() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path().join("d");

        // `rootPath` is a plain string, not a URI, so it is taken as-is.
        let roots = workspace_roots_from(serde_json::json!({
            "processId": null,
            "rootUri": null,
            "rootPath": root.to_str().unwrap(),
            "capabilities": {},
        }));

        assert_eq!(roots, vec![root]);
    }

    #[test]
    fn test_server_capabilities_wire_format() {
        let session = create_test_session();
        let capabilities = serde_json::to_value(session.server_capabilities()).unwrap();

        assert_eq!(capabilities["positionEncoding"], "utf-16");
        assert_eq!(capabilities["textDocumentSync"]["openClose"], true);
        assert_eq!(capabilities["textDocumentSync"]["change"], 2);
        assert_eq!(
            capabilities["textDocumentSync"]["save"]["includeText"],
            false
        );
        assert_eq!(
            capabilities["codeActionProvider"]["codeActionKinds"][0],
            "quickfix"
        );
        assert_eq!(capabilities["codeActionProvider"]["resolveProvider"], false);
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
        let uri = Uri::parse("file:///test.py").unwrap();
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

    /// The `contentChanges` array exactly as a client sends it.
    fn content_changes(json: serde_json::Value) -> Vec<TextDocumentContentChangeEvent> {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn test_update_document_applies_content_changes() {
        let mut session = create_test_session();
        let uri = Uri::parse("file:///test.R").unwrap();
        session.open_document(uri.clone(), TextDocument::new("hello world".to_string(), 1));

        session
            .update_document(
                uri.clone(),
                content_changes(serde_json::json!([{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 5 }
                    },
                    "text": "hi"
                }])),
                2,
            )
            .unwrap();

        let document = session.get_document(&uri).unwrap();
        assert_eq!(document.content(), "hi world");
        assert_eq!(document.version(), 2);
    }

    #[test]
    fn test_update_document_uses_session_position_encoding() {
        // The session negotiated UTF-16, so the character offsets in the change
        // are UTF-16 code units: the emoji is two of them but four bytes.
        let mut session = create_test_session();
        assert_eq!(session.position_encoding(), PositionEncoding::UTF16);

        let uri = Uri::parse("file:///test.R").unwrap();
        session.open_document(uri.clone(), TextDocument::new("🌍 world".to_string(), 1));

        session
            .update_document(
                uri.clone(),
                content_changes(serde_json::json!([{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 2 }
                    },
                    "text": "hi"
                }])),
                2,
            )
            .unwrap();

        assert_eq!(session.get_document(&uri).unwrap().content(), "hi world");
    }

    #[test]
    fn test_update_document_rejects_whole_document_replacement() {
        let mut session = create_test_session();
        let uri = Uri::parse("file:///test.R").unwrap();
        session.open_document(uri.clone(), TextDocument::new("hello world".to_string(), 1));

        let error = session
            .update_document(
                uri.clone(),
                content_changes(serde_json::json!([{ "text": "replaced" }])),
                2,
            )
            .unwrap_err();

        assert!(
            error.to_string().contains("Full document replacement"),
            "unexpected error: {error}"
        );
        let document = session.get_document(&uri).unwrap();
        assert_eq!(document.content(), "hello world");
        assert_eq!(document.version(), 1);
    }

    #[test]
    fn test_update_and_close_unknown_document_fail() {
        let mut session = create_test_session();
        let uri = Uri::parse("file:///missing.R").unwrap();

        let error = session.update_document(uri.clone(), vec![], 2).unwrap_err();
        assert!(
            error.to_string().contains("Document not found"),
            "unexpected error: {error}"
        );

        let error = session.close_document(uri.clone()).unwrap_err();
        assert!(
            error.to_string().contains("Document not found"),
            "unexpected error: {error}"
        );

        assert!(session.take_snapshot(uri).is_none());
    }

    #[test]
    fn test_snapshot_position_conversions() {
        let mut session = create_test_session();
        let uri = Uri::parse("file:///test.R").unwrap();
        session.open_document(
            uri.clone(),
            TextDocument::new("hello\nworld".to_string(), 1),
        );

        let snapshot = session.take_snapshot(uri.clone()).unwrap();
        assert_eq!(snapshot.uri(), &uri);

        assert_eq!(snapshot.position_to_offset(Position::new(1, 0)).unwrap(), 6);
        assert_eq!(snapshot.offset_to_position(6).unwrap(), Position::new(1, 0));
        assert_eq!(
            snapshot.range_of_span(0, 5).unwrap(),
            Range::new(Position::new(0, 0), Position::new(0, 5))
        );
    }

    #[test]
    fn test_position_encoding_negotiation() {
        // Test UTF-8 preference
        let mut caps = ClientCapabilities {
            general: Some(GeneralClientCapabilities {
                position_encodings: Some(vec![
                    PositionEncodingKind::UTF8,
                    PositionEncodingKind::UTF16,
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };

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

        if let Some(TextDocumentSync::Options(options)) = caps.text_document_sync {
            assert_eq!(options.open_close, Some(true));
            assert_eq!(options.change, Some(TextDocumentSyncKind::Incremental));
        }
    }

    #[test]
    fn test_config_notification_shown_for_parent_config() {
        use std::fs;

        let mut session = create_test_session();

        // Create a temporary directory structure with a config file in parent
        let temp_dir = tempfile::TempDir::new().unwrap();
        let parent_dir = temp_dir.path();
        let child_dir = parent_dir.join("subdir");
        fs::create_dir_all(&child_dir).unwrap();

        // Create a jarl.toml in the parent directory
        let config_path = parent_dir.join("jarl.toml");
        fs::write(&config_path, "[lint]\n").unwrap();

        // Create a test file in the child directory
        let test_file = child_dir.join("test.R");
        fs::write(&test_file, "x <- 1\n").unwrap();

        // First call should show notification (config is in parent dir, not CWD)
        let result1 = session.check_and_notify_config_with_cwd(&test_file, &child_dir);

        // Second call should not show notification again (flag is set)
        let result2 = session.check_and_notify_config_with_cwd(&test_file, &child_dir);

        // Now run assertions
        assert!(result1, "Notification should be shown on first occurrence");
        assert!(
            session.config_notification_shown,
            "Flag should be set when notification is shown"
        );
        assert!(!result2, "Notification should not be shown twice");
    }

    #[test]
    fn test_config_notification_flag_prevents_duplicate() {
        let mut session = create_test_session();

        // Initially, notification should not be shown
        assert!(!session.config_notification_shown);

        // Manually set the flag to simulate notification already shown
        session.config_notification_shown = true;

        // Create a test file (won't matter since flag is already set)
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.R");
        std::fs::write(&test_file, "x <- 1\n").unwrap();

        // Even if there's a config to discover, it should return false
        let result = session.check_and_notify_config(&test_file);

        assert!(
            !result,
            "Notification should not be shown when flag is already set"
        );
        assert!(session.config_notification_shown, "Flag should remain true");
    }

    #[test]
    fn test_config_notification_not_shown_for_cwd_config() {
        use std::fs;

        let mut session = create_test_session();

        // Create a temporary directory with a config file in CWD
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cwd = temp_dir.path();

        // Create a jarl.toml in the current directory
        let config_path = cwd.join("jarl.toml");
        fs::write(&config_path, "[lint]\n").unwrap();

        // Create a test file in the same directory
        let test_file = cwd.join("test.R");
        fs::write(&test_file, "x <- 1\n").unwrap();

        // Should not show notification for config in CWD
        let result = session.check_and_notify_config_with_cwd(&test_file, cwd);

        // Notification should not be shown for CWD config
        assert!(
            !result,
            "Notification should not be shown for config in CWD"
        );
        assert!(
            !session.config_notification_shown,
            "Flag should not be set for CWD config"
        );
    }

    #[test]
    fn test_config_notification_with_no_config() {
        use std::fs;

        let mut session = create_test_session();

        // Create a temporary directory without a config file
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cwd = temp_dir.path();

        // Create a test file without any config
        let test_file = cwd.join("test.R");
        fs::write(&test_file, "x <- 1\n").unwrap();

        // Should not show notification when no config exists
        let result = session.check_and_notify_config_with_cwd(&test_file, cwd);

        // Notification should not be shown when no config exists
        assert!(
            !result,
            "Notification should not be shown when no config exists"
        );
        assert!(
            !session.config_notification_shown,
            "Flag should not be set when no config exists"
        );
    }
}
