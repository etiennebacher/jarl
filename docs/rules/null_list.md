# null_list

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `sapply()`, `lapply()` and `purrr::map()` calls where the
anonymous function returns only `NULL`.

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

The automated fix is marked unsafe because `vector("list", length(x))`
doesn't preserve names: `lapply()` and `purrr::map()` keep the names of
`x`, and `sapply()` also uses `x` itself as names when it is a character
vector. Jarl cannot know whether `x` has names, so they need to be added
back manually if they matter, e.g. with
`setNames(vector("list", length(x)), names(x))`.
