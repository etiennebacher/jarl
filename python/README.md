# jarl-linter

An R linter. Written in Rust.

This package provides the `jarl` command-line tool as a Python package, making it easy to install via `pip` or `uv`.

## Installation

Install a global installation of `jarl` with [uv](https://docs.astral.sh/uv/):

```bash
uv tool install jarl-linter
```

or with pip

```bash
pip install jarl-linter
```

This puts `jarl` on the PATH, so you can run:

```bash
# Format R file
jarl check .
```

Alternatively, invoke jarl via `uvx` for one-off formatting without a global install:

```bash
uvx --from jarl-linter jarl check .
```

To use a specific version of jarl:

```bash
# Global install
uv tool install jarl-linter@0.4.0

# One off runs
uvx --from jarl-linter@0.4.0 jarl check .
```

## About

For more information, see the [main repository](https://github.com/etiennebacher/jarl).
