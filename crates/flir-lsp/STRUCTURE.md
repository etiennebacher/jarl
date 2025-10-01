# Flir LSP Project Structure

This document outlines the complete structure of the `flir_lsp` crate and explains how each component works together to provide LSP functionality for the Flir linter.

## Directory Structure

```
flir_lsp/
├── Cargo.toml                 # Crate configuration and dependencies
├── README.md                  # User documentation and setup guide
├── STRUCTURE.md              # This file - architectural overview
├── src/
│   ├── lib.rs                # Main library entry point and server wrapper
│   ├── lint.rs               # Core linting integration with flir_core
│   └── bin/
│       └── flir-lsp.rs       # CLI binary for running the LSP server
├── examples/
│   └── cli_integration.rs    # Example of integrating LSP into main CLI
└── tests/
    └── integration_tests.rs  # Integration tests for linting functionality
```

## Component Overview

### Core Library (`src/lib.rs`)

**Purpose**: Main entry point that orchestrates the LSP server

**Key Responsibilities**:
- Initialize the LSP server using Ruff's infrastructure
- Set up logging and worker threads
- Provide the `FlirServer` wrapper around Ruff's `Server`
- Handle graceful startup and shutdown

**Key Types**:
- `FlirServer`: Wrapper that customizes Ruff's server for Flir
- Re-exports from `ruff_server`: `Session`, `Client`, `DocumentSnapshot`, etc.

### Linting Integration (`src/lint.rs`)

**Purpose**: Bridge between Flir's linting engine and the LSP protocol

**Key Responsibilities**:
- Convert document content to Flir's input format
- Run Flir's linting analysis
- Transform Flir diagnostics to LSP diagnostic format
- Handle position encoding (UTF-8, UTF-16, UTF-32)
- Manage diagnostic severity mapping

**Key Functions**:
- `check_document()`: Main entry point for linting a document
- `run_flir_linting()`: Interface to your `flir_core` crate
- `convert_flir_diagnostic_to_lsp()`: Format conversion
- `line_col_to_position()`: Position encoding handling

**Integration Points** (to be implemented):
```rust
// Replace mock implementation with:
use flir_core::{Linter, LintResult, Diagnostic as FlirDiagnostic};

fn run_flir_linting(content: &str, file_path: Option<&Path>) -> Result<Vec<FlirDiagnostic>> {
    let linter = Linter::new(/* config */);
    let results = linter.analyze(content, file_path)?;
    Ok(results.diagnostics)
}
```

### CLI Binary (`src/bin/flir-lsp.rs`)

**Purpose**: Standalone executable for running the LSP server

**Features**:
- Command-line argument parsing
- Logging configuration (level, file output)
- Stdio communication with editors
- Error handling and process exit codes

**Usage**:
```bash
flir-lsp --stdio --log-level debug --log-file /tmp/flir.log
```

## Data Flow

### 1. Server Initialization
```
Editor → LSP Initialize Request → FlirServer → Ruff's Server Infrastructure
  ↓
Capability Negotiation (diagnostics, encoding, etc.)
  ↓
Server Ready, Main Loop Started
```

### 2. Document Linting Flow
```
Editor Opens/Changes File
  ↓
ruff_server handles LSP protocol
  ↓
Document content extracted
  ↓
lint::check_document() called
  ↓
flir_core linting engine runs
  ↓
Diagnostics converted to LSP format
  ↓
Published to editor via LSP notifications
```

### 3. Real-time Updates
```
User Types in Editor
  ↓
DidChange notification sent
  ↓
Document state updated in Session
  ↓
Re-linting triggered automatically
  ↓
New diagnostics published
  ↓
Editor highlights issues in real-time
```

## Integration with Your Existing Crates

### With `flir_core`

The `lint.rs` module is designed to be a thin adapter. You'll need to:

1. **Replace mock types** with actual `flir_core` types:
   ```rust
   // Remove MockFlirDiagnostic, use your actual type:
   use flir_core::Diagnostic;
   ```

2. **Implement the linting bridge**:
   ```rust
   fn run_flir_linting(content: &str, file_path: Option<&Path>) -> Result<Vec<Diagnostic>> {
       let config = load_flir_config(file_path)?; // Your config loading logic
       let linter = flir_core::Linter::new(config);
       let results = linter.lint_str(content, file_path)?;
       Ok(results)
   }
   ```

3. **Map diagnostic fields** to LSP format in `convert_flir_diagnostic_to_lsp()`

### With `flir_cli`

See `examples/cli_integration.rs` for adding an LSP subcommand:

```rust
// Add to your existing CLI enum:
#[derive(Subcommand)]
enum Commands {
    Check { /* existing */ },
    Fix { /* existing */ },
    Lsp {  // <-- New LSP command
        #[arg(long, default_value = "info")]
        log_level: String,
    },
}
```

## Features Inherited from Ruff

By leveraging `ruff_server`, you automatically get:

### LSP Protocol Handling
- ✅ **Document Lifecycle**: `textDocument/didOpen`, `didChange`, `didClose`
- ✅ **Diagnostics**: Both push notifications and pull requests
- ✅ **Initialization**: Proper LSP handshake and capability negotiation
- ✅ **Error Handling**: Robust error responses and logging

### Advanced Capabilities
- ✅ **Incremental Sync**: Only re-analyze changed portions
- ✅ **Multi-threading**: Background linting doesn't block LSP
- ✅ **Position Encoding**: UTF-8/UTF-16/UTF-32 support
- ✅ **Workspace Support**: Multi-folder projects
- ✅ **Configuration**: File watching and dynamic reloading

### Performance Features
- ✅ **Caching**: Document content and settings cached
- ✅ **Debouncing**: Rapid changes don't trigger excessive re-linting
- ✅ **Memory Management**: Proper cleanup of closed documents

## Testing Strategy

### Unit Tests (`src/lint.rs`)
- Test diagnostic conversion logic
- Test position encoding handling
- Test severity mapping

### Integration Tests (`tests/integration_tests.rs`)
- Test complete linting pipeline
- Test multiple document scenarios
- Test error handling

### Manual Testing
```bash
# Build and test
cargo test

# Test LSP communication manually
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' \
  | cargo run --bin flir-lsp -- --stdio

# Test with real editor
# Configure your editor to use: target/debug/flir-lsp --stdio
```

## Deployment Options

### 1. Standalone Binary
Package `flir-lsp` as a separate executable that editors can invoke.

### 2. Integrated Subcommand
Add LSP functionality to your main `flir` CLI:
```bash
flir lsp --stdio  # Instead of flir-lsp --stdio
```

### 3. Editor Extensions
Create editor-specific packages that bundle the LSP server.

## Next Steps

1. **Replace Mock Implementation**: 
   - Remove `MockFlirDiagnostic` and related mock types
   - Import actual types from `flir_core`
   - Implement real linting bridge

2. **Add Configuration Support**:
   - Load Flir configuration files
   - Support workspace-specific settings
   - Handle dynamic configuration updates

3. **Extend Features**:
   - Add code actions (quick fixes)
   - Add hover information
   - Add completion support (if applicable)

4. **Testing**:
   - Test with real editors (VS Code, Neovim, etc.)
   - Performance testing with large files
   - Edge case handling

This structure provides a solid foundation that leverages Ruff's mature LSP infrastructure while allowing you to focus on integrating your specific linting logic.