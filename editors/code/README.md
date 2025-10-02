# Flir VS Code Extension

A Visual Studio Code extension that provides linting support for R code through the Flir language server.

## Features

- **Real-time linting**: Get instant feedback on R code quality issues as you type
- **Diagnostic messages**: Clear, actionable error and warning messages
- **Configurable rules**: Enable/disable specific linting rules through configuration
- **Multi-workspace support**: Works across different R projects and workspaces

## Installation

### From VSIX (Development)

1. Build the extension:
   ```bash
   cd editors/code
   npm install
   npm run package
   ```

2. Install the generated `.vsix` file:
   ```bash
   code --install-extension flir-vscode-*.vsix
   ```

### From Marketplace

*Coming soon - extension will be published to the VS Code marketplace.*

## Requirements

The extension requires the Flir language server binary. The extension will automatically:

1. Try to use a bundled binary (if available)
2. Look for `flir` in your system PATH
3. Use a custom path if configured

## Configuration

Configure the extension through VS Code settings:

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

## Supported File Types

The extension activates for:
- `.R` files
- `.r` files  
- `.Rprofile` / `.rprofile` files
- R projects (directories containing `.Rproj` files)
- Projects with `flir.toml` or `pyproject.toml` configuration

## Commands

- **Flir: Restart Server** - Restart the language server

Access commands via `Ctrl+Shift+P` (Cmd+Shift+P on macOS) and search for "Flir".

## Configuration Files

Flir looks for configuration in:
- `flir.toml` in project root
- `pyproject.toml` with `[tool.flir]` section
- Parent directories (recursive search)

Example `flir.toml`:
```toml
[tool.flir]
select = ["PERF", "READ001"]
ignore = ["READ002"]

[tool.flir.per-file-ignores]
"scripts/" = ["READ001"]
```

## Troubleshooting

### Extension Not Working

1. Check the Output panel (`View > Output`) and select "Flir Language Server"
2. Verify flir binary is installed and accessible
3. Try restarting the language server: `Ctrl+Shift+P` → "Flir: Restart Server"

### Binary Not Found

If you see "Failed to find executable" errors:

1. Install flir: `cargo install --git https://github.com/etiennebacher/flir2`
2. Or set a custom path: `"flir.executablePath": "/path/to/flir"`
3. Or use bundled binary: `"flir.executableStrategy": "bundled"`

### No Diagnostics Showing

1. Ensure you're working with `.R` files
2. Check that flir configuration allows the current file
3. Look for configuration files that might be excluding rules

## Development

### Building from Source

```bash
git clone https://github.com/etiennebacher/flir2
cd flir2/editors/code
npm install
npm run compile
```

### Packaging

```bash
npm run package  # Creates .vsix file
```

## License

MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test the extension
5. Submit a pull request

## Links

- [Flir Project](https://github.com/etiennebacher/flir2)
- [VS Code Extension Development](https://code.visualstudio.com/api)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)