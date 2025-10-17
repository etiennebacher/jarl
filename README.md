# jarl

`jarl` is a fast linter for R, written in Rust. It is built upon Air, a fast formatter.

## Installation

> [!WARNING]
> While the repo is private, I recommend not using the binaries provided in the Releases. Those are not
> frequently updated due to limitations in Github Actions on private repos.
>
> For now, the best way to get the binary is to clone the repo and build the binary from source with
> `cargo install --path . --profile=release`

macOS and Linux:
```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/etiennebacher/jarl2/releases/download/v0.0.17/jarl-installer.sh | sh
```

Windows:

```
powershell -ExecutionPolicy Bypass -c "irm https://github.com/etiennebacher/jarl2/releases/download/v0.0.17/jarl-installer.ps1 | iex"
```

Alternatively, if you have Rust installed, you can get the development version with:

```
cargo install --git https://github.com/etiennebacher/jarl2
```

## Acknowledgements

* [`lintr` authors and contributors](https://lintr.r-lib.org/authors.html): while the infrastructure is completely different, all the rule definitions and a large part of the tests are inspired or taken from `lintr`.
* Davis Vaughan and Lionel Henry, both for their work on Air and for their advices and answers to my questions during the development of `jarl`.
* R Consortium for funding part of the development of `jarl`.


![](r-consortium-logo.png)
