_default:
    just --list

# Update the list of rules and the website
document:
  Rscript -e 'source("docs/make_docs.R")'
  (cd docs && quarto render)

# Run cargo clippy and cargo fmt
lint:
  cargo clippy \
    --all-targets \
    --all-features \
    --locked \
    -- \
    -D warnings \
    -D clippy::dbg_macro

  cargo fmt

# Apply fixes reported by `just lint`
lint-fix:
  cargo clippy \
    --all-targets \
    --all-features \
    --locked \
    --fix --allow-dirty

  cargo fmt

# Generates the `jarl.schema.json`
gen-schema:
    cargo run -p xtask_codegen -- json-schema

# Builds the release binary, copy it, and builds the extension
build-install-positron-extension:
  cargo build --release
  cp target/release/jarl editors/code/bundled/bin/jarl
  cd editors/code && rm -rf *.vsix && vsce package && positron --install-extension *.vsix


# Copies the release binary and builds the extension
install-positron-extension:
  cp target/release/jarl editors/code/bundled/bin/jarl
  cd editors/code && rm -rf *.vsix && vsce package && positron --install-extension *.vsix
