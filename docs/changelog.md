# Changelog

## Development

### Features

- Add support for `list2df` rule (#179).
- Add support for `browser` rule (#185, @jonocarroll).
- Add support for `system_file` rule (#186).

### Fixes

- (Hopefully) Fixed wrong printing of ANSI characters in multiple terminals on Windows (#179, thanks @novica for the report).

### Documentation

- Added sections on RStudio and Helix to the [Editors](https://jarl.etiennebacher.com/editors) page.
- Added installation instructions using Scoop on Windows.

## 0.1.1

### Fixes

- Fix discovery of `jarl.toml` by the Jarl extension (#175, thanks @DavisVaughan for the report).
- Rule `duplicated_argument` no longer reports `cli_` functions where multiple arguments have the same name (#176, thanks @DavisVaughan for the report).

### Documentation

- The docs of `assignment` rule now explain how to change the preferred assignment operator.

## 0.1.0

First release (announced)
