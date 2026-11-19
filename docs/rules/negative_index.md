# negative_index

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `[]` subsetting with non-literal negative indices, such as
variables or calls (`x[-idx]`, `x[-which(x == 99)]`), as well as negated
zero indices (`x[-0]`, or `x[-idx]` when `idx` is known to be `0`).

This rule does not report:

- Named arguments in `[]`, such as `roll = -tol` in `dt[x, roll = -tol]`.
- Non-zero literals, constant arithmetic, and `:` sequences, such as `x[-1]`,
  `x[-(2 * 3)]`, `x[-c(1, 2)]`, and `x[-(1:n)]`.
- Calls to `c()` with a known positive subscript, such as `x[-c(idx, 1)]`,
  `x[-c(idx, c(1, 2))]`, or `x[-c(idx, 1:3)]`.
- Subsetting by the same object's length or dimension, such as
  `x[-length(x)]`, `df[-nrow(df), ]`, `df[, -ncol(df)]`, or `df[-NCOL(df)]`.
- Group size indices, such as `x[-n()]`, `x[-dplyr::n()]`, and `dt[-.N, ]`.
  `n()` must have no arguments and be the sole index. `.N` must be the sole
  index or the row index.
- An unchanged `for` loop variable drawn from `seq_along()`, `seq_len()`,
  `which()`, or a range of positive integer literals such as `1:10`.
- `-idx` guarded by an `if` or `while` condition proving `idx` is nonempty,
  such as `length(idx) > 0`, `length(idx)`, or `any(idx > 0)`.
- `-idx` after a guard that exits when `idx` is empty (such as `return()`,
  `stop()`, `rlang::abort()`, `cli::cli_abort()`, `next`, or `break`), or
  after an assertion such as `stopifnot(length(idx) > 0)`.
- `-idx` after assigning a non-zero constant or a `:` sequence, such as
  `idx <- 1`, `idx <- 2 * 3`, or `idx <- a:b`.

## Why is this bad?

If `idx` is empty or `0`, `x[-idx]` selects no elements instead of keeping
all of `x`. For example, trying to remove a value that is not present
returns an empty vector:

```r
x <- 1:10
x[-which(x == 99)]
#> integer(0)
```

Since `99` is not in `x`, the intended result is the unchanged vector `1:10`
rather than an empty vector.

No automatic fix is provided. Handle empty or zero indices explicitly or use
a logical index when possible.

## Example

```r
x <- 1:10
x[-which(x == 99)]
```

Use instead:

```r
x <- 1:10
x[x != 99]
```
