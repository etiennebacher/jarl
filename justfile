_default:
    just --list

# Update the rules pages and build the website in `docs/_site/` (`--preview` to serve it locally instead, `--no-render` to skip the website)
[arg("preview", long="preview", value="true")]
[arg("no_render", long="no-render", value="true")]
document preview="false" no_render="false":
    Rscript docs/make_docs.R
    @if [ "{{ no_render }}" = "true" ]; then :; elif [ "{{ preview }}" = "true" ]; then uvx zensical serve --open --config-file docs/zensical.toml; else uvx zensical build --clean --config-file docs/zensical.toml; fi

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
build-install-positron-extension: && install-positron-extension
  cargo build --release

# Copies the release binary and builds the extension
install-positron-extension:
  mkdir -p editors/code/bundled/bin
  cp target/release/jarl editors/code/bundled/bin/jarl
  cd editors/code && { [ -d node_modules ] || npm ci; }
  cd editors/code && rm -rf *.vsix && vsce package && positron --install-extension *.vsix

# Install the jarl binary (release mode) to `~/.cargo/bin/jarl`.
# Note that a `~/.local/bin/jarl` installed another way may shadow this.
install-binary:
    cargo install --path crates/jarl --force --profile=release
