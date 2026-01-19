<div align="center"><h1>jarl</h1></div>
<div align="center"><i>Just Another R Linter</i> </div>
<br>
<div align="center">
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
* Support for 40+ rules (and growing)
* Integration in popular IDEs and editors (VS Code, Positron, Zed, ...)
* CLI available
* Multiple output modes (concise, detailed, JSON format)
* CI workflow

Jarl is built on [Air](https://posit-dev.github.io/air/), a fast formatter for R written in Rust.

<br>

[^benchmark]: Using 20 rules on the `dplyr` package (~25k lines of R code), Jarl took 0.131s, `flir` took 4.5s, and `lintr` took 18.5s (9s with caching enabled).


## Quick start

You can use Jarl manually via the command line, or use [extensions](https://jarl.etiennebacher.com/editors) to have it integrated in your coding environment.

This shows how to use it in the terminal:

`test.R`:
```r
any(is.na(x))

if (all.equal(x, y)) {
  print("x and y are equal")
}
```

```
# In the terminal:
$ jarl check test.R
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: all_equal
 --> test.R:3:5
  |
3 | if (all.equal(x, y)) {
  |     --------------- `all.equal()` can return a string instead of FALSE.
  |
  = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if
    no tolerance is required.

Found 2 errors.
1 fixable with the `--fix` option (1 hidden fix can be enabled with the
`--unsafe-fixes` option).
```

Use `--fix` to automatically fix rule violations when possible:

```sh
$ jarl check test.R --fix
```

`test.R`:
```r
anyNA(x)

if (all.equal(x, y)) {
  print("x and y are equal")
}
```


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
