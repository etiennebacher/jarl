<div align="center"><h1>jarl</h1></div>
<div align="center"><i>Just Another R Linter</i> </div>

<br>

Jarl is a fast linter[^linter-explanation] for R.

* XXX times faster than `lintr` and XXX times faster than `flir`
* automatic fixes when possible
* support for 20+ [`lintr` rules](https://jarl.etiennebacher.com/rules) (and growing)
* integration in popular IDEs and editors (VS Code, Positron, Emacs, Vim, ...)
* CLI available
* built-in CI workflow

Jarl is built on [Air](https://posit-dev.github.io/air/), a fast formatter for R written in Rust.

<br>

[^linter-explanation]: A code linter is a tool that does static code analysis to search for programming errors, bugs, and suspicious patterns of code.

## Installation

### Binaries

**macOS and Linux:**

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/etiennebacher/jarl/releases/download/v0.0.17/jarl-installer.sh | sh
```

**Windows:**

```sh
powershell -ExecutionPolicy Bypass -c \
  "irm https://github.com/etiennebacher/jarl/releases/download/v0.0.17/jarl-installer.ps1 | iex"
```

### From source

Alternatively, if you have Rust installed, you can get the development version with:

```sh
cargo install --git https://github.com/etiennebacher/jarl --profile=release
```

## Acknowledgements

* [`lintr` authors and contributors](https://lintr.r-lib.org/authors.html): while the infrastructure is completely different, all the rule definitions and a large part of the tests are inspired or taken from `lintr`.
* Davis Vaughan and Lionel Henry, both for their work on Air and for their advices and answers to my questions during the development of `jarl`.
* R Consortium for funding part of the development of `jarl`.
* the design of Jarl is heavily inspired by [Ruff](https://docs.astral.sh/ruff) and [Cargo clippy](https://doc.rust-lang.org/stable/clippy/).

![](r-consortium-logo.png)
