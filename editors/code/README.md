# Jarl VS Code Extension

A Visual Studio Code extension that provides linting support for R code through the Jarl language server.

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
   cd /path_to/jarl2/editors/code
   cp ../../target/release/jarl bundled/bin/jarl # don't forget to use target/debug if used `cargo build`
   npm install
   npm run package
   ```

2. Install the generated `.vsix` file:
   ```bash
   positron --install-extension jarl-vscode-*.vsix
   ```


## Requirements

The extension requires the Jarl language server binary. The extension will automatically:

1. Try to use a bundled binary (if available)
2. Look for `jarl` in your system PATH
3. Use a custom path if configured

## Configuration

Configure the extension through Positron / VS Code settings:

### Basic Settings

- `jarl.logLevel`: Set the log level for the language server (`error`, `warning`, `info`, `debug`, `trace`)
- `jarl.executableStrategy`: How to locate the jarl binary (`bundled`, `environment`, `path`)
- `jarl.executablePath`: Custom path to jarl binary (when using `path` strategy)

### Example Configuration

```json
{
  "jarl.logLevel": "info",
  "jarl.executableStrategy": "environment",
  "jarl.executablePath": "/path/to/custom/jarl"
}
```

## Commands

- **Jarl: Restart Server** - Restart the language server

Access commands via `Ctrl+Shift+P` (Cmd+Shift+P on macOS) and search for "Jarl".
