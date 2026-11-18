# unnecessary_lambda

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for anonymous functions in `lapply()`, `sapply()`, `purrr::map()`,
and similar functions, that only call another function, e.g.
`lapply(x, function(xi) sum(xi))`.

It also checks for anonymous functions in `sapply()` and `vapply()` that
only compare the result of a function to a constant, e.g.
`sapply(x, function(xi) foo(xi) == 2)`.

## Why is this bad?

The anonymous function is unnecessary: the function it calls can be passed
directly, e.g. `lapply(x, sum)`.

Comparing to a constant once on the output of `sapply()` or `vapply()`,
e.g. `sapply(x, foo) == 2`, is faster than doing it for every element since
`==` is vectorized.

Anonymous functions whose first argument isn't the iterated element, e.g.
`lapply(x, function(xi) grep("a", xi))`, or that pass extra arguments to the
inner function, e.g. `lapply(x, function(xi) sum(xi, na.rm = TRUE))` or
`lapply(x, function(xi, y) foo(xi, y), y)`, are not reported because removing
them is not always more readable.

This rule has an unsafe automatic fix when the anonymous function only
passes its arguments to the inner function. It is unsafe because the inner
function may use non-standard evaluation, e.g. `function(x) substitute(x)`.

## Example

```r
lapply(x, function(xi) sum(xi))
purrr::map(x, ~ sum(.x))
sapply(x, function(xi) foo(xi) == 2)
```

Use instead:
```r
lapply(x, sum)
purrr::map(x, sum)
sapply(x, foo) == 2
```
