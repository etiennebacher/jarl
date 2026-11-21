# unnecessary_lambda

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for anonymous functions in `lapply()`, `sapply()`, `purrr::map()`,
and similar functions, that only call another function, e.g.
`lapply(x, function(xi) sum(xi))`.

## Why is this bad?

The anonymous function is unnecessary: the function it calls can be passed
directly, e.g. `lapply(x, sum)`.

Anonymous functions that do more than calling another function, e.g.
`sapply(x, function(xi) foo(xi) == 2)`, are not reported. Neither are
anonymous functions whose first argument isn't the iterated element, e.g.
`lapply(x, function(xi) grep("a", xi))`, or that pass extra arguments to the
inner function, e.g. `lapply(x, function(xi) sum(xi, na.rm = TRUE))` or
`lapply(x, function(xi, y) foo(xi, y), y)`, because removing them is not
always more readable.

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
