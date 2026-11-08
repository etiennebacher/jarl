# apply_paste

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for usage of `apply(x, 1, paste, collapse = ...)` to paste together
the columns of each row.

## Why is this bad?

`apply()` coerces its input to a matrix and calls `paste()` once per row,
which is slow. Since `paste()` is vectorized, the same result can be
obtained in a single call with `do.call(paste, c(x, sep = ...))`, which is
both faster and clearer.

The automated fix is marked unsafe because the result of
`do.call(paste, c(x, sep = ...))` depends on the type and content of `x`.
It can differ from the original call when:

- `x` is a matrix: `c(x, sep = ...)` is not a list, so `do.call()` errors.
- `x` is a `data.frame` mixing character and non-character columns:
  `apply()` converts it with `as.matrix()`, which pads numbers and logicals
  to a common width (e.g. `" 1_x"` instead of `"1_x"`).
- `x` has a column named `sep`, `collapse`, or `recycle0`: it is matched to
  the argument of `paste()` with the same name, which either errors or
  changes the output.
- `x` has non-automatic row names: `apply()` uses them to name the output,
  `do.call(paste, ...)` does not.
- `collapse` is a variable that evaluates to `NULL`: `apply()` doesn't
  collapse rows, while `c()` drops `sep = NULL` so `paste()` uses its
  default `sep = " "`.

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
