# stopifnot_all
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for direct calls to `all()` inside `stopifnot()`.

## Why is this bad?

`stopifnot()` already checks that all values of each argument are `TRUE`.
Wrapping an argument in `all()` is therefore unnecessary.

## Example

```r
stopifnot(all(x > 0))
```

Use instead:
```r
stopifnot(x > 0)
```

## References

See `?stopifnot`.
