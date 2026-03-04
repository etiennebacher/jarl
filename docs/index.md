---
toc: false
title: Jarl
---

<div style="text-align: center !important"><i>Just Another R Linter</i> </div>
<br>
<div style="text-align: center !important">
  <a href = "https://jarl.etiennebacher.com/" target = "_blank"><img src="https://img.shields.io/static/v1?label=Docs&message=Visit&color=blue"></a>
  <a href = "https://github.com/etiennebacher/jarl/actions" target = "_blank"><img src="https://github.com/etiennebacher/jarl/workflows/cargo-test/badge.svg"></a>
  <a href="https://codecov.io/gh/etiennebacher/jarl" >
  <img src="https://codecov.io/gh/etiennebacher/jarl/graph/badge.svg?token=P859N5VE46"/>
  </a>
</div>

<br>

Jarl is a fast linter for R: it does static code analysis to search for programming errors, bugs, and suspicious patterns of code.

* Orders of magnitude faster than `lintr` and `flir`[^benchmark]
* Automatic fixes when possible
* Support for 55+ rules (and growing)
* Integration in popular IDEs and editors (VS Code, Positron, Zed, ...)
* Command-line interface (CLI)
* Multiple output modes (concise, detailed, JSON format)
* CI workflow

Jarl is built on [Air](https://posit-dev.github.io/air/), a fast formatter for R written in Rust.

<br>

[^benchmark]: Using 20 rules on the `dplyr` package (~25k lines of R code), Jarl took 0.131s, `flir` took 4.5s, and `lintr` took 18.5s (9s with caching enabled).


## Installation

### Released version

Either get binaries from the [Releases page](https://github.com/etiennebacher/jarl/releases) or install Jarl from the existing installer scripts below.

**macOS and Linux:**

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/etiennebacher/jarl/releases/latest/download/jarl-installer.sh | sh
```

**Windows:**

```sh
powershell Set-ExecutionPolicy Bypass -Scope Process -Force; `
   iwr https://github.com/etiennebacher/jarl/releases/latest/download/jarl-installer.ps1 | iex
```

If you use Scoop, you can also install or update Jarl with [these commands](https://github.com/cderv/r-bucket#jarl):

```sh
scoop bucket add r-bucket https://github.com/cderv/r-bucket.git

# install
scoop install jarl

# update
scoop update jarl
```

### Development version

Some pre-releases may be available from the [Releases page](https://github.com/etiennebacher/jarl/releases) (the version usually contains `alpha`).

Alternatively, if you have Rust installed, you should be able to get the development version with:

```sh
cargo install --git https://github.com/etiennebacher/jarl jarl --profile=release
```

## Next steps

- [Tour](tour.md) — a quick walkthrough of Jarl's main features
- [By Example](by-example.md) — short recipes for common tasks
- [Editor setup](howto/editors.md) — integrate Jarl in your IDE

## Related work

[`lintr`](https://lintr.r-lib.org/) is the most famous R linter.
It provides dozens of rules related to performance, readibility, formatting, and more.
Jarl is heavily influenced by `lintr` since most rule definitions come from it.
However, `lintr` doesn't provide automatic fixes for rule violations, which makes it harder to use.
Its performance also noticeably degrades as the number of files and their length increase.

[`flir`](https://flir.etiennebacher.com/) is a relatively novel package.
It uses [`ast-grep`](https://ast-grep.github.io/) in the background to search and replace code patterns.
It is therefore quite flexible and easy to extend by users who may want more custom rules.
While both Jarl and `ast-grep` use [`tree-sitter`](https://tree-sitter.github.io/tree-sitter/) in the background to parse R files, their structure is completely different.
Jarl is faster and also easier to link to the Language Server Protocol, which enables its use via VS Code or Positron extensions for instance.


## Acknowledgements

* [`lintr` authors and contributors](https://lintr.r-lib.org/authors.html): while the infrastructure is completely different, all the rule definitions and a large part of the tests are inspired or taken from `lintr`.
* Davis Vaughan and Lionel Henry, both for their work on Air and for their advices and answers to my questions during the development of Jarl.
* the design of Jarl is heavily inspired by [Ruff](https://docs.astral.sh/ruff) and [Cargo clippy](https://doc.rust-lang.org/stable/clippy/).
* R Consortium for funding part of the development of Jarl.

<img src="r-consortium-logo.png" alt="R Consortium logo" width="30%"/>
