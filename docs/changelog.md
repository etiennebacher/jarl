# Changelog

## Development

### Features

- Added support for `seq` and `seq2` rules (#187).
- Added support for `expect_true_false` rule. This is a bit experimental because it is the first rule that is not enabled by default (and doesn't have a minimum R version requirement) (#191).

### Documentation

- Added section on Neovim to the [Editors](https://jarl.etiennebacher.com/editors) page (#188, @bjyberg).
- Added page "Tutorial: add a new rule" (#183).

## 0.1.2

### Features

- Added support for `list2df` rule (#179).
- Added support for `browser` rule (#185, @jonocarroll).
- Added support for `system_file` rule (#186).

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
