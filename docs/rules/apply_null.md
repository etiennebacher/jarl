# apply_null

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `sapply()`, `lapply()` and `purrr::map()` calls where the
anonymous function returns only `NULL`.

Calls to `map()` without the `purrr::` prefix are only reported when
`map()` can be attributed to purrr (e.g. after `library(purrr)`).

## Why is this bad?

These calls iterate over the input only to create a list of `NULL` values.
Creating an empty list of a given length with `vector()` is more efficient.

## Example

```r
lapply(x, function(x) NULL)
purrr::map(x, \(x) NULL)
purrr::map(x, ~NULL)
```

Use instead:
```r
vector("list", length(x))
```

This suggestion does not preserve names from `x`.
