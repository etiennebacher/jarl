# Jarl VS Code Extension

A Visual Studio Code extension for [Jarl](https://jarl.etiennebacher.com/), a fast R linter.

## Example Configuration

```json
{
  "jarl.logLevel": "info",
  "jarl.executableStrategy": "environment",
  "jarl.executablePath": "/path/to/custom/jarl",
}
```

## More information

See the website: https://jarl.etiennebacher.com/

## Changelog

### 0.0.10

- Removed the setting "assignment" from the VS Code / Positron settings. This
  should be defined in a `jarl.toml` file. To use this by default on all R files,
  even those outside of a specific project, create a `jarl.toml` in [your config
  folder](https://jarl.etiennebacher.com/config#config-file-detection).
  (https://github.com/etiennebacher/jarl/pull/257)
