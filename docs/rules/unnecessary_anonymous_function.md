# unnecessary_anonymous_function

::: {.callout-note title="Added in development version" .low-opacity}
:::

## What it does

Checks for anonymous functions in `lapply()`, `sapply()`, `purrr::map()`,
and similar functions, that only call another function, e.g.
`lapply(x, function(xi) sum(xi))`.

## Why is this bad?

The anonymous function is unnecessary: the function it calls can be passed
directly, e.g. `lapply(x, sum)`.

The following cases are not reported:

- anonymous functions that do more than calling another function, e.g.
  `sapply(x, function(xi) foo(xi) == 2)`
- anonymous functions whose first argument isn't the iterated element, e.g.
  `lapply(x, function(xi) grep("a", xi))`
- anonymous functions that pass extra arguments to the inner function, e.g.
  `lapply(x, function(xi) sum(xi, na.rm = TRUE))` or
  `lapply(x, function(xi, y) foo(xi, y), y)`

This rule has an unsafe automatic fix because the inner function may use
non-standard evaluation, e.g. `function(x) substitute(x)`.

## Example

```r
lapply(x, function(xi) sum(xi))
purrr::map(x, ~ sum(.x))
```

Use instead:
```r
lapply(x, sum)
purrr::map(x, sum)
```
