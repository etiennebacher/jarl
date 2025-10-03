# Flir LSP

A minimal Language Server Protocol (LSP) implementation focused purely on providing real-time lint diagnostics. No code actions, formatting, or other advanced features - just highlighting issues as you type.

## Overview

This crate provides a lightweight, diagnostics-only LSP server for the Flir linter. It's designed to be simple, fast, and easy to integrate with your existing linter, providing real-time feedback in editors and IDEs.

## Features

- ✅ **Real-time Diagnostics**: Instant lint feedback as you type
- ✅ **Lightweight**: Minimal dependencies, fast startup
- ✅ **Multi-threaded**: Non-blocking background linting
- ✅ **Position Encoding**: UTF-8, UTF-16, and UTF-32 support
- ✅ **Incremental Updates**: Efficient handling of document changes
- ✅ **Push & Pull Diagnostics**: Both legacy and modern diagnostic modes
- ✅ **Easy Integration**: Simple bridge to your existing linter

## What This LSP Server Does

- **Highlights lint issues** in your editor as you type
- **Shows diagnostic messages** when you hover over issues
- **Updates diagnostics** automatically when you change files
- **Handles multiple files** and workspaces

## What This LSP Server Does NOT Do

- ❌ Code actions or quick fixes
- ❌ Code formatting
- ❌ Auto-completion
- ❌ Hover information
- ❌ Multi-file analysis
- ❌ Refactoring tools

This keeps the implementation simple and focused on the core use case: showing lint issues in real-time.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Editor/IDE    │    │   flir_lsp      │    │   flir_core     │
│                 │    │                 │    │                 │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │   LSP     │◄─┼────┼─►│   Server  │  │    │  │   Your    │  │
│  │  Client   │  │    │  │           │◄─┼────┼─►│  Linter   │  │
│  └───────────┘  │    │  └───────────┘  │    │  └───────────┘  │
│                 │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Quick Start

### 1. Integration with Your Linter

The main integration point is in `src/lint.rs`. Replace the mock implementation with calls to your actual linting engine:

```rust
// In src/lint.rs, replace the run_flir_linting function:
use flir_core::{Linter, Config};

fn run_flir_linting(content: &str, file_path: Option<&Path>) -> Result<Vec<FlirDiagnostic>> {
    // Load your linter configuration
    let config = Config::load_from_path(file_path)?;
    
    // Create and run your linter
    let linter = Linter::new(config);
    let results = linter.lint_text(content, file_path)?;
    
    // Convert your diagnostics to the FlirDiagnostic format
    Ok(results.into_iter().map(|d| FlirDiagnostic {
        message: d.message,
        severity: match d.level {
            flir_core::Level::Error => FlirSeverity::Error,
            flir_core::Level::Warning => FlirSeverity::Warning,
            // ... etc
        },
        line: d.span.start.line,
        column: d.span.start.column,
        end_line: d.span.end.line,
        end_column: d.span.end.column,
        code: Some(d.rule_code),
        rule_name: Some(d.rule_name),
    }).collect())
}
```

### 2. Build and Run

```bash
# Build the LSP server
cargo build --bin flir-lsp

# Run the server
./target/debug/flir-lsp

# Or install and run
cargo install --path .
flir-lsp --log-level debug
```

### 3. Editor Configuration

#### VS Code

Configure in your `settings.json`:

```json
{
  "flir.serverPath": "/path/to/flir-lsp",
  "flir.trace.server": "verbose"
}
```

Or create a VS Code extension with this configuration:

```json
{
  "languageServerExample.serverPath": "/path/to/flir-lsp"
}
```

#### Neovim (with nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Configure the Flir LSP server
configs.flir = {
  default_config = {
    cmd = {'flir-lsp'},
    filetypes = {'r'},  -- R language files
    root_dir = lspconfig.util.root_pattern('.git', 'flir.toml', '.Rprofile'),
    settings = {},
  },
}

lspconfig.flir.setup{}
```

#### Emacs (with lsp-mode)

```elisp
(require 'lsp-mode)

(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "flir-lsp")
                  :major-modes '(r-mode ess-r-mode)
                  :server-id 'flir-lsp))
```

## CLI Usage

### Standalone Binary

```bash
# Basic usage - connects via stdio
flir-lsp

# With debug logging to file
flir-lsp --log-level debug --log-file /tmp/flir-lsp.log

# See all options
flir-lsp --help
```

### Integrated with Your Main CLI

See `examples/cli_integration.rs` for a complete example of adding LSP support to your existing CLI:

```rust
#[derive(Subcommand)]
enum Commands {
    Check { /* your existing check command */ },
    Lsp {  // Add this LSP subcommand
        #[arg(long, default_value = "info")]
        log_level: String,
        #[arg(long)]
        log_file: Option<String>,
    },
}

// In your command handler:
Commands::Lsp { log_level, log_file } => {
    setup_lsp_logging(&log_level, log_file.as_deref())?;
    flir_lsp::run()
}
```

Then users can run:
```bash
flir lsp --log-level debug
```

## Configuration

### Environment Variables

- `FLIR_LSP_MAX_THREADS`: Maximum worker threads (default: CPU count, max 4)
- `FLIR_LSP_LOG_LEVEL`: Logging level (error, warn, info, debug, trace)
- `FLIR_LSP_LOG_FILE`: Log to file instead of stderr

### Runtime Configuration

The server automatically:
- Detects client capabilities during initialization
- Negotiates the best position encoding (UTF-8 > UTF-16 > UTF-32)
- Configures diagnostic mode (push vs pull) based on client support
- Sets up incremental document synchronization

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Test specific module
cargo test lint

# With output for debugging
cargo test -- --nocapture
```

### Manual Testing

1. Build and test basic LSP communication:
   ```bash
   cargo build --bin flir-lsp
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' \
     | ./target/debug/flir-lsp
   ```

2. Test with debug logging:
   ```bash
   ./target/debug/flir-lsp --log-level debug --log-file /tmp/debug.log
   ```

3. In another terminal, watch the logs:
   ```bash
   tail -f /tmp/debug.log
   ```

## Troubleshooting

### Server not starting
```bash
# Check if binary exists
ls -la target/debug/flir-lsp
./target/debug/flir-lsp --help
```

### No diagnostics appearing
```bash
# Enable debug logging to see what's happening
RUST_LOG=debug flir-lsp --log-file /tmp/debug.log

# Check your editor's LSP client configuration
# Look for initialization messages in the debug log
```

### Position encoding issues
- Make sure your Flir diagnostics use byte offsets (not character offsets)
- Test with files containing Unicode characters
- Check the negotiated encoding in debug logs

### Performance issues
```bash
# Reduce worker threads
FLIR_LSP_MAX_THREADS=1 flir-lsp
```

## Implementation Details

### Core Components

1. **`Server`** (`src/server.rs`): Handles LSP protocol messages
2. **`Session`** (`src/session.rs`): Manages document state and client capabilities  
3. **`Client`** (`src/client.rs`): Sends messages to the editor
4. **`Document`** (`src/document.rs`): Document lifecycle and position encoding
5. **`Lint`** (`src/lint.rs`): **Your integration point** - bridge to flir_core

### Message Flow

```
1. Editor opens file → didOpen notification → Server
2. Server stores document → triggers background linting
3. Linter runs → converts results to LSP diagnostics
4. Server sends publishDiagnostics → Editor shows highlights
5. User edits file → didChange notification → re-lint → update diagnostics
```

### Diagnostic Modes

**Push Diagnostics (for older editors):**
- Server automatically sends diagnostic notifications
- Happens on file open and every change

**Pull Diagnostics (for modern editors):**
- Editor requests diagnostics when needed
- More efficient, better for large files

The server automatically detects which mode to use based on client capabilities.

## Dependencies

Minimal and focused:
- `lsp-server` & `lsp-types`: LSP protocol implementation
- `crossbeam`: Multi-threading for background linting
- `anyhow`: Error handling
- `tracing`: Logging
- `serde`: JSON serialization

No heavyweight frameworks or unnecessary dependencies.

## License

This implementation is designed to be a clean foundation for your Flir linter's LSP integration.