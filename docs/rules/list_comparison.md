# list_comparison
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for comparisons involving functions that are known to return lists,
such as `lapply(x, sum) > 10`.

## Why is this bad?

These functions return lists, so R must coerce their results to vectors
before comparing them. This can hide the intended output type. Prefer a
mapper that returns a vector of the intended type directly, such as
`vapply()` or one of the typed `purrr::map_*()` functions.

This rule doesn't have an automatic fix because the expected output type
cannot be determined reliably from static analysis.

## Example

```r
lapply(x, sum) > 10
map(x, as.character) == "a"
```

Use instead:
```r
vapply(x, sum, numeric(1L)) > 10
map_chr(x, as.character) == "a"
```

## References

See `?lapply` and `?Comparison`.
