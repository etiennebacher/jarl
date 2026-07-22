# apply_paste
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for usage of `apply(x, 1, paste, collapse = ...)` to paste together
the columns of each row.

## Why is this bad?

`apply()` coerces its input to a matrix and calls `paste()` once per row,
which is slow. Since `paste()` is vectorized, the same result can be
obtained in a single call with `do.call(paste, c(x, sep = ...))`, which is
both faster and clearer.

The automated fix is marked unsafe because `do.call(paste, c(x, sep = ...))`
only reproduces the original result when `x` is a `data.frame` (a list of
columns). For a plain matrix, `c(x, sep = ...)` flattens all elements
instead of keeping one entry per column, so the fix would change the result.

## Example

```r
apply(df[, c("x", "y")], 1, paste, collapse = "_")
```

Use instead:
```r
do.call(paste, c(df[, c("x", "y")], sep = "_"))
```

## References

See `?do.call` and `?paste`
