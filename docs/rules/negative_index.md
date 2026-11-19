# negative_index

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `[]` subsetting with non-literal negative indices, such as
variables or calls (`x[-idx]`, `x[-which(x == 99)]`), as well as negated
zero indices (`x[-0]`, or `x[-idx]` when `idx` is known to be `0`).

This rule does not report:

- Named arguments in `[]` (e.g., `dt[x, roll = -tol]`).
- Non-zero literals, constant arithmetic, non-zero `:` sequences, and `c()`
  calls with a positive subscript (e.g., `x[-1]`, `x[-(1:n)]`,
  `x[-c(idx, 1)]`).
- The subsetted object's own length or dimensions (e.g., `x[-length(x)]`,
  `df[-nrow(df), ]`), or group size helpers (`x[-n()]`, `dt[-.N]`).
- Unmodified `for` loop variables iterating over `seq_along()`, `seq_len()`,
  `which()`, or a positive integer range (e.g., `1:10`).
- Variables guarded against being empty by an `if`/`while` condition, an
  early exit, or `stopifnot()` (e.g., `if (length(idx) > 0) x[-idx]`).
- Variables previously assigned a non-zero constant, non-zero `:` sequence,
  or positive `c()` call (e.g., `idx <- c(1, 3); x[-idx]`).

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

# Or handle empty indices explicitly when you already have an index vector:
idx <- which(x == 99)
if (length(idx) > 0) x[-idx] else x
```
