# deparse1
::: {.callout-note title="Added in 0.6.1" .low-opacity}
:::

## What it does

Checks for usage of `paste(deparse(x), collapse = " ")`.

## Why is this bad?

Since R 4.0.0, it is possible to do `paste(deparse(x), collapse = " ")`
with `deparse1(x)`, which is more efficient and easier to read.

This rule comes with an unsafe fix because `deparse1()`'s default
`width.cutoff` (500) differs from `deparse()`'s (60), so the two calls
can produce different output when `width.cutoff` isn't set explicitly.
This rule is only enabled if the project explicitly uses R >= 4.0.0 (or
if the argument `--min-r-version` is passed with a version >= 4.0.0).

## Example

```r
paste(deparse(args(library)), collapse = " ")
```

Use instead:
```r
deparse1(args(library))
```

## References

See `?deparse1`
