<div align="center"><h1>jarl</h1></div>
<div align="center"><i>Just Another R Linter</i> </div>
<br>
<div align="center">
  <a href = "https://jarl.etiennebacher.com/" target = "_blank"><img src="https://img.shields.io/static/v1?label=Docs&message=Visit&color=blue"></a>
  <a href = "https://github.com/etiennebacher/jarl/actions" target = "_blank"><img src="https://github.com/etiennebacher/jarl/workflows/cargo-test/badge.svg"></a>
</div>

<br>

Jarl is a fast linter for R: it does static code analysis to search for programming errors, bugs, and suspicious patterns of code.

* Orders of magnitude faster than `lintr` and `flir`[^benchmark]
* Automatic fixes when possible
* Support for 20+ [`lintr` rules](https://jarl.etiennebacher.com/rules) (and growing)
* Integration in popular IDEs and editors (VS Code, Positron, Emacs, Vim, ...)
* CLI available
* Multiple output modes (concise, detailed, JSON format)
* CI workflow

Jarl is built on [Air](https://posit-dev.github.io/air/), a fast formatter for R written in Rust.

<br>

[^benchmark]: Using 20 rules on the `dplyr` package (~25k lines of R code), Jarl took 0.131s, `flir` took 4.5s, and `lintr` took 18.5s (9s with caching enabled).


## Quick start

You can use Jarl via the command line.

`test.R`:
```r
any(is.na(x))
```

```sh
$ jarl check test.R
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

Found 1 error.
1 fixable with the `--fix` option.
```

Use `--fix` to automatically fix rule violations when possible:

```sh
$ jarl check test.R --fix
```

`test.R`:
```r
anyNA(x)
```

Jarl can also be directly integrated in your coding environment, see [Editors](https://jarl.etiennebacher.com/editors).


## Installation

### Binaries

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

### From source

Alternatively, if you have Rust installed, you can get the development version with:

```sh
cargo install --git https://github.com/etiennebacher/jarl --profile=release
```

## Acknowledgements

* [`lintr` authors and contributors](https://lintr.r-lib.org/authors.html): while the infrastructure is completely different, all the rule definitions and a large part of the tests are inspired or taken from `lintr`.
* Davis Vaughan and Lionel Henry, both for their work on Air and for their advices and answers to my questions during the development of Jarl.
* the design of Jarl is heavily inspired by [Ruff](https://docs.astral.sh/ruff) and [Cargo clippy](https://doc.rust-lang.org/stable/clippy/).
* R Consortium for funding part of the development of Jarl.

![](r-consortium-logo.png){width="30%"}
