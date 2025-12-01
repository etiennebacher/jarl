# Changelog

## Development

### Breaking changes

Now when running `jarl` by default it will exclude the files defined in the
default file exclude patterns. A new cli argument `--no-default-exclude` in
the CLI can be used to check those files as well. The argument overrides the
`default-exclude=true` option when set in `jarl.toml`. (#178)

### Features

- `--output-format json` now contains two fields `diagnostics` and `errors` (#219).
- Better support for namespaced function calls, both when reporting violations
  and when fixing them (#221).

### Changes

- `expect_named` no longer reports cases like `expect_equal(x, names(y))` because
  rewriting those as `expect_named(y, x)` would potentially change the intent of
  the test and the way it is read (#220).

## 0.2.1

### Other

- Important performance improvement when using `--fix`, in particular in projects with many R files (#217).

## 0.2.0

### Breaking changes

- For consistency between CLI arguments and `jarl.toml` arguments, the following CLI arguments are renamed (#199):
  - `--select-rules` becomes `--select`
  - `--ignore-rules` becomes `--ignore`
  - `--assignment-op` becomes `--assignment`

### Features

- New argument `extend-select` in `jarl.toml` and `--extend-select` in the CLI to select additional rules on top of the existing selection. This can be useful to select opt-in rules in addition to the default set of rules (#193).
- Added support for `seq` and `seq2` rules (#187).
- Added support for several rules related to `testthat`. Those rules are disabled by default and can be enabled by combining `select` or `extend-select` with the rule name or the `TESTTHAT` group rule name. Those rules are:
  - `expect_length` (#211)
  - `expect_named` (#212)
  - `expect_not` (#204)
  - `expect_null` (#202)
  - `expect_true_false` (#191)

### Fixes

- `implicit_assignment` no longer reports cases inside `quote()` (#209).

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
