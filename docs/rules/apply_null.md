# apply_null

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `sapply()` and `lapply()` calls where the anonymous function
returns only `NULL`.

## Why is this bad?

These calls iterate over the input only to create a list of `NULL` values.
Allocating the list directly is more efficient.

## Example

```r
lapply(x, function(x) NULL)
```

Use instead:
```r
vector("list", length(x))
```

This suggestion does not preserve names from `x`.
