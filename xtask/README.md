# xtask

This directory contains Rust scripts for code generation.

The only usage for now is to regenerate the `artifacts/jarl.schema.json` that is used by the Tombi extension for autocompletion of `jarl.toml`.

Re-generate this file with `cargo run -p xtask_codegen -- json-schema`.
