# Project Structure

This document describes the overall architecture and organization of the Flir project, a Rust-based linter for R code.

## Overview

Flir is organized as a Rust workspace with multiple crates, each serving a specific purpose in the linting pipeline. The project follows a modular architecture inspired by Ruff, with clear separation between core linting logic, command-line interface, and language server protocol implementation.

## Workspace Structure

```
flir2/
├── crates/                 # Rust workspace crates
│   ├── flir-core/         # Core linting engine
│   ├── flir-cli/          # Command-line interface
│   └── flir-lsp/          # Language Server Protocol implementation
├── docs/                  # Documentation (Quarto-based)
├── demos/                 # Example R files for testing
├── target/                # Rust build artifacts
└── Cargo.toml            # Workspace configuration
```

## Core Components

### flir-core

The heart of the linting system, responsible for:

- **AST Analysis**: Parsing and traversing R code abstract syntax trees
- **Rule Definitions**: Individual lint rules with their logic and fixes
- **Diagnostic Generation**: Creating structured diagnostic messages
- **Fix Application**: Applying code fixes to resolve lint violations
- **Configuration Management**: Handling settings, rule tables, and project configuration

**Key modules:**
- `analyze/`: AST traversal and rule application logic
- `lints/`: Individual rule implementations
- `config.rs`: Configuration parsing and validation
- `diagnostic.rs`: Diagnostic message structures
- `fix.rs`: Fix application and conflict resolution
- `settings.rs`: Runtime settings management

### flir-cli

Command-line interface providing:

- **File Discovery**: Finding R files in directories and projects
- **Batch Processing**: Running lints across multiple files
- **Output Formatting**: Displaying results in various formats
- **Fix Mode**: Automatically applying fixes to files
- **Configuration**: Command-line argument parsing

**Key modules:**
- `commands/`: Subcommand implementations (check, fix, server)
- `args.rs`: CLI argument definitions
- `status.rs`: Exit status handling

### flir-lsp

Language Server Protocol implementation for editor integration:

- **Real-time Diagnostics**: Live linting as you type
- **Document Management**: Tracking file changes and versions
- **Position Encoding**: Handling different text encoding formats
- **Client Communication**: Bidirectional LSP message handling

**Key modules:**
- `server.rs`: Main LSP server loop
- `session.rs`: Session state and workspace management
- `client.rs`: LSP client communication
- `document.rs`: Document lifecycle and content tracking
- `lint.rs`: Integration with flir-core for diagnostics

## Data Flow

### 1. Configuration Discovery

```
Project Root → pyproject.toml/flir.toml → Settings → RuleTable
```

The system discovers configuration files in the project hierarchy, parses them into structured settings, and builds a rule table determining which rules are enabled.

### 2. File Discovery

```
Input Paths → File Traversal → Filtering → R File List
```

Given input paths, the system recursively discovers R files while respecting ignore patterns and configuration.

### 3. Linting Pipeline

```
R File → AST Parsing → Rule Analysis → Diagnostics → Output/Fixes
```

For each R file:
1. Parse into an AST using `air_r_parser`
2. Traverse the AST, applying enabled rules at appropriate nodes
3. Collect diagnostics with location information
4. Either output diagnostics or apply fixes

### 4. Rule Application

```
AST Node → Rule Matcher → Diagnostic Generation → Optional Fix
```

Each rule:
1. Matches specific AST node patterns
2. Analyzes the node for violations
3. Generates diagnostics with precise source locations
4. Optionally provides automated fixes

## Rule System

### Rule Organization

Rules are organized by categories:
- **PERF**: Performance-related issues
- **READ**: Readability improvements
- **STYLE**: Code style consistency
- **CORRECTNESS**: Potential bugs or errors

### Rule Structure

Each rule follows a consistent pattern:

```rust
pub struct RuleName;

impl Rule for RuleName {
    fn check(&self, node: &AstNode, context: &Context) -> Vec<Diagnostic> {
        // Rule logic here
    }
    
    fn fix(&self, diagnostic: &Diagnostic) -> Option<Fix> {
        // Optional fix generation
    }
}
```

### Adding New Rules

1. Define the rule in `flir-core/src/lints/`
2. Add rule metadata to `flir-core/src/lints/mod.rs`
3. Implement rule logic with tests
4. Register rule in appropriate AST analyzer (e.g., `analyze/expression.rs`)
5. Add documentation and examples

## Integration Points

### Editor Integration (LSP)

The LSP server provides real-time feedback to editors:
- Diagnostics appear as you type
- Quick fixes available via code actions
- Configuration changes reflected immediately

### CI/CD Integration (CLI)

The CLI tool integrates into build pipelines:
- Exit codes indicate lint status
- Multiple output formats (JSON, SARIF, human-readable)
- Fix mode for automated code cleanup

### Library Integration

flir-core can be used as a library:
- Programmatic access to linting functionality
- Custom rule development
- Integration into other R tooling

## Configuration System

### Hierarchy

Configuration is resolved in order of precedence:
1. Command-line arguments
2. `flir.toml` in current directory
3. `pyproject.toml` [tool.flir] section
4. Parent directory configuration (recursive)
5. Default settings

### Settings Structure

```toml
[tool.flir]
# Rule selection
select = ["PERF", "READ001"]
ignore = ["READ002"]

# File patterns
include = ["*.R", "*.Rmd"]
exclude = ["tests/"]

# Rule-specific configuration
[tool.flir.per-file-ignores]
"scripts/" = ["READ001"]
```

## Testing Strategy

### Unit Tests
- Rule-specific tests in each rule module
- Snapshot tests for fix verification
- Configuration parsing tests

### Integration Tests
- End-to-end CLI behavior
- LSP protocol compliance
- Multi-file project scenarios

### Performance Tests
- Large codebase handling
- Memory usage profiling
- Parallel processing efficiency

## Development Workflow

### Building
```bash
cargo build                    # Debug build
cargo build --release         # Optimized build
```

### Testing
```bash
cargo test                     # Run all tests
cargo insta test               # Snapshot tests
cargo insta review            # Review snapshot changes
```

### Installation
```bash
cargo install --path crates/flir-cli --profile release
```

## Future Considerations

### Extensibility
- Plugin system for custom rules
- External rule packages
- Rule configuration templates

### Performance
- Incremental analysis
- Better caching strategies
- Parallel rule execution

### Language Features
- R package-aware analysis
- Cross-file dependency analysis
- Type inference integration

This structure provides a solid foundation for R code linting while maintaining flexibility for future enhancements and integrations.