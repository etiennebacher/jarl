# flir

`flir` is a fast linter for R, written in Rust. It is built upon Air, a fast formatter.

## Installation

TODO: doesn't work while the repo is private

macOS and Linux:
```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/etiennebacher/flir2/releases/download/v0.0.15/flir-installer.sh | sh
```

Windows:

```
powershell -ExecutionPolicy Bypass -c "irm https://github.com/etiennebacher/flir2/releases/download/v0.0.15/flir-installer.ps1 | iex"
```

Alternatively, if you have Rust installed, you can get the development version with:

```
cargo install --git https://github.com/etiennebacher/flir2
```

## Acknowledgements

* Davis Vaughan and Lionel Henry, both for their work on Air and for their advices and answers to my questions during the development of `flir`.
* R Consortium for funding part of the development of `flir`.


![](r-consortium-logo.png)
