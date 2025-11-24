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
