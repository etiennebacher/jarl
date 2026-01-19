---
title: Contributing to Jarl
---

## Tools

Jarl is written in Rust, so you must [install the Rust toolchain](https://rust-lang.org/tools/install/).

Jarl relies on Insta to run snapshot tests. You can install it with:
```sh
cargo install cargo-insta
```

Jarl uses [just](https://github.com/casey/just) to quickly run several useful commands, such as `just document` or `just lint` (see the list of commands with `just -l`).
This is not mandatory but convenient.

## Basic structure of the repository

The folder `crates` contains several sub-crates.
At the time of writing (October 2025), there are three:

* `jarl` contains the structure of the command-line tool with which users interact. This is where we can add or modify arguments to be passed to the CLI.
* `jarl-core` contains the "meat" of the linter. It is where the R code is parsed and checked, and where the rules are defined. **This is probably the crate you will have to modify.**
* `jarl-lsp` contains the code to integrate the linter with the Language Server Protocol, which allows editors such as VS Code or Positron to highlight diagnostics and provide "Quick Fix" buttons for example.

## Adding or modifying a rule

In this section, all paths refer to files in `crates/jarl-core`.

### List of existing rules

`src/lints/mod.rs` contains the existing list of rules.
Each rule must have a name, belong to one or several categories (`PERF`, `READ`, etc.), a `FixStatus` indicating whether it has a fix and, if so, whether this fix is safe or unsafe, and an optional minimum required R version.

### Lint definition

`src/lints` contains the definition of the rules, along with their associated documentation and tests. It has one subfolder per rule and two mandatory files: `<rule_name>.rs` (which contains the definition and documentation) and `mod.rs` (which contains the tests).

If there are snapshot tests for this rule, then a subfolder `snapshots` will also be created.
For example, the folder for the rule `any_duplicated` looks like this:

```sh
src/lints/any_duplicated/
├── any_duplicated.rs
├── mod.rs
└── snapshots
    └── jarl__lints__any_duplicated__tests__fix_output.snap
```

### Adding a new rule

Adding a new rule requires four main steps:

1. Add the new rule to the list in `src/lints/mod.rs`. In the same file, also add `pub(crate) mod <rulename>;`
1. Add a subfolder with the rule name in `src/lints`. Add the documentation and the code for the rule.
1. Add tests in `src/lints/<rulename>/mod.rs`
1. Add the rule in the `src/analyze` folder. This depends on the initial node in the AST. For instance, for the rule `equals_na`, we check the presence of code such as `x == NA`. Since the top node for this expression is a `R_BINARY_EXPRESSION`, this rule is ran in `src/analyze/binary_expression.rs`.

### Useful commands

* `cargo run --bin jarl -- check file.R` (or any other paths to check). The `--` in the middle is required to use the CLI in development mode (i.e. without installing it with `cargo install`)
* `cargo insta test` and `cargo insta review` (if necessary) for snapshot tests only.
* `cargo test` to run all tests, including snapshot tests.
* `cargo install --path crates/jarl --profile=release` (or `--profile=dev`) to have a system-wide install and test the crate in other R projects.


## Integration tests

When you add a new rule, it is usually sufficient to add tests in the directory of this rule only, e.g. in `crates/jarl-core/src/lints/any_duplicated`.

However, in some cases you may affect the way users interact with Jarl as a whole, for instance by adding new command line arguments or arguments that can be set in `jarl.toml`. For those changes, it is important to check that the general behavior of Jarl is correct (check what happens when there are no R files, how TOML and CLI arguments interact, etc.).

Tests for this are stored in `crates/jarl-cli/tests/integration`.
It is likely that you will need to edit one of the files instead of creating a new one.
For example, adding an extra argument in `crates/jarl-core/toml.rs` would require adding tests in `crates/jarl-cli/tests/integration/toml.rs`.


## PR title

Jarl follows [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#summary), meaning that your PR must start with "feat:", "fix:", or another appropriate name (see the linked documentation).
See the [list of previous PRs](https://github.com/etiennebacher/jarl/pulls?q=is%3Apr+is%3Aclosed+) for some examples.
