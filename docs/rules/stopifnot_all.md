# stopifnot_all
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for direct calls to `all()` inside `stopifnot()`.

## Why is this bad?

`stopifnot()` already checks `all()` of each argument internally. Passing
`all(x)` hides the original expression from `stopifnot()`, which results in
a less informative error message when the condition fails.

## Example

```r
stopifnot(all(x > 0))
```

Use instead:
```r
stopifnot(x > 0)
```

## References

See the [`lintr` rule](https://lintr.r-lib.org/reference/stopifnot_all_linter.html)
and `?stopifnot`.
