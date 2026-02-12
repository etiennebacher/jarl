# Changelog

## Development version

### Features

* `jarl.toml` now accepts rule-specific options in subsections `[lint.<rule-name>]`,
  such as:
  ```
  [lint]
  ...

  # In the rule `duplicated_arguments`, do not report calls to `my_function()`
  # and `list()` where multiple arguments have the same name.
  [lint.duplicated-arguments]
  extend-skipped-functions = ["my_function", "list"]
  ```
  These options are listed in the [Configuration page](https://jarl.etiennebacher.com/config#with-a-config-file) (#333).

## 0.4.0

### Breaking changes

- The support for comments to hide violations (aka *suppression comments*, aka
  `# nolint` comments in `lintr`) has been entirely reworked (#218). It is no
  longer compatible with `lintr`'s comments. Instead, Jarl now uses `# jarl-ignore`
  comments and follows different rules regarding the syntax and location of those
  comments. Detailed documentation is available in the section
  ["Ignoring diagnostics"](https://jarl.etiennebacher.com/using-jarl#ignoring-diagnostics)
  on the website. As part of this rewrite, the following rules have been added:

  - `blanket_suppression` (#243)
  - `misnamed_suppression` (#309)
  - `misplaced_file_suppression` (#305)
  - `misplaced_suppression` (#307)
  - `unexplained_suppression` (#304)
  - `unmatched_range_suppression` (#312)
  - `unused_suppression` (#310)

### Features

- New CLI argument `--statistics` to show the number of violations per rule instead
  of the details of each violation. Jarl prints a hint to use this argument when
  more than 15 violations are reported (only when `--output-format` is `concise`
  or `full`). This value can be configured with the environment variable
  `JARL_N_VIOLATIONS_HINT_STAT`. (#250, #266)

- Jarl now looks in parent folders for `jarl.toml`. It searches until the user
  config folder is reached (the location of this folder depends on the OS:
  `~/.config` on Unix and `~/AppData/Roaming` on Windows). Jarl uses the first
  `jarl.toml` that is found. This is useful to store settings that should be
  common to all projects (e.g. `assignment = "<-"`) without creating a
  `jarl.toml`, which is a common situation for standalone R scripts. (#253)

- New rules:
  - `equals_nan` (#284)
  - `equals_null` (#283)
  - `for_loop_dup_index` (#327)
  - `if_always_true` (#311, @bjyberg)
  - `internal_function` (#291)
  - `redundant_ifelse` (#260)
  - `unnecessary_nesting` (#268)
  - `unreachable_code` (#261)

- When the output format is `full` or `concise`, rule names now have a hyperlink
  leading to the website documentation (#278).

- `any_is_na` now reports `NA %in% x` (#286).

### Other changes

- The following rules are now disabled by default. They still exist and the user
  can choose to use them, but they were deemed too noisy for limited benefit to
  be enabled by default:
  - `assignment` (#258)
  - `fixed_regex` (#279)
  - `sample_int` (#262)

- `equals_na` now reports `x %in% NA` cases, as documented (#285).

- There are now binaries available for `linux-musl` (`x64` and `arm64`) (#287).

### Bug fixes

- When `output-format` is `json` or `github`, additional information displayed in
  the terminal (e.g. timing) isn't included anymore to avoid parsing errors (#254).

- Fixed a bug in the number of "fixable diagnostics" reported when the arg
  `fixable` is present in `jarl.toml` but `--fix` is not passed (#255).

- `fixed_regex` is now correctly classified as "Performance" instead of
  "Readability" rule internally (#279).

- Default values of function parameters are now analyzed too (#282).

- `duplicated_arguments` doesn't report anymore cases where argument names `"`
  and `'` were conflated, e.g.

  ```r
  switch(x, `"` = "double", `'` = "single")
  ```
  (#288).

### Documentation

- New section in the `Integrations` page to show how to use Jarl in various
  CI/CD platforms (#289, @philipp-baumann).

## 0.3.0

### Breaking changes

- Jarl now excludes by default file paths matching the following patterns:
  `.git/`, `renv/`, `revdep/`, `cpp11.R`, `RcppExports.R`, `extendr-wrappers.R`,
  and `import-standalone-*.R`.

  A new CLI argument `--no-default-exclude` can be used to check those files as
  well. This argument overrides the `default-exclude = true` option when set in
  `jarl.toml` (#178, @novica).

### Features

- `--output-format json` now contains two fields `diagnostics` and `errors` (#219).
- Better support for namespaced function calls, both when reporting violations
  and when fixing them (#221).
- The `class_equals` rule now also reports cases like `identical(class(x), "foo")`
  and `identical("foo", class(x))` (#234).
- New rules:
  - `expect_s3_class` (#235)
  - `expect_type` (#226)
  - `fixed_regex` (#227)
  - `sprintf` (#224)
  - `string_boundary` (#225)
  - `vector_logic` (#238)

### Fixes

- `# nolint` comments are now properly applied to nodes that are function arguments, e.g.
  ```r
  foo(
    # nolint
    any(is.na(x))
  )
  ```
  does not report a violation anymore (#229).

### Other changes

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
