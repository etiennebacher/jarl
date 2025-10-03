# Flir VS Code Extension

A Visual Studio Code extension that provides linting support for R code through the Flir language server.

## Features

- **Real-time linting**: Get instant feedback on R code quality issues as you type
- **Diagnostic messages**: Clear, actionable error and warning messages
- **Configurable rules**: Enable/disable specific linting rules through configuration
- **Multi-workspace support**: Works across different R projects and workspaces

## Installation

### From VSIX (dev on Positron)

1. Build the extension:
   ```bash
   cargo build --release
   cd /path_to/flir2/editors/code
   cp ../../target/release/flir bundled/bin/flir # don't forget to use target/debug if used `cargo build`
   npm install
   npm run package
   ```

2. Install the generated `.vsix` file:
   ```bash
   positron --install-extension flir-vscode-*.vsix
   ```


## Requirements

The extension requires the Flir language server binary. The extension will automatically:

1. Try to use a bundled binary (if available)
2. Look for `flir` in your system PATH
3. Use a custom path if configured

## Configuration

Configure the extension through Positron / VS Code settings:

### Basic Settings

- `flir.logLevel`: Set the log level for the language server (`error`, `warning`, `info`, `debug`, `trace`)
- `flir.executableStrategy`: How to locate the flir binary (`bundled`, `environment`, `path`)
- `flir.executablePath`: Custom path to flir binary (when using `path` strategy)

### Example Configuration

```json
{
  "flir.logLevel": "info",
  "flir.executableStrategy": "environment",
  "flir.executablePath": "/path/to/custom/flir"
}
```

## Commands

- **Flir: Restart Server** - Restart the language server

Access commands via `Ctrl+Shift+P` (Cmd+Shift+P on macOS) and search for "Flir".
