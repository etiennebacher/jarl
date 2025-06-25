# flir

WIP for extending `flir` capacities.

```sh
cargo run
```

With this in `foo2.R`:

```r
mean(is.na(x))

a <- 2
any(is.na(x))

b <- T
```
then it returns this:

```shell
foo2.R [3:8] any-na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
foo2.R [6:6] T-F-symbols `T` and `F` can be confused with variable names. Spell `TRUE` and `FALSE` entirely instead.
```
