# rep_times_ignored
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for `rep()` calls that supply both `times` and `length.out`.

## Why is this bad?

When both arguments are supplied, `length.out` takes priority and `times`
is ignored. This likely indicates a mistake in the call.

This rule is disabled by default and has an unsafe fix because
`length.out` can evaluate to `NA` or another invalid value, in which case
`times` is still used.

## Example

```r
rep(1:3, times = 2, length.out = 10)
```

Use instead:
```r
rep(1:3, length.out = 10)
```

## References

See `?rep`
