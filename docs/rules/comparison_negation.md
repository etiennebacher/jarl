# comparison_negation
::: {.callout-note title="Added in 0.0.23" .low-opacity}
:::

## What it does

Checks for patterns similar to `!(... < ...)`.

## Why is this bad?

This pattern may be hard to read and could be simplified by removing the `!`
operator and inverting the operator (e.g. `<` would become `>=`).

This rule has an unsafe fix because of operator precedence around the
comparison:

```r
x <- 1
y <- 2

2 * !(x < y)
#> [1] 0
2 * x >= y
#> [1] TRUE
```

## Example

```r
!(x < y + 1)
!(x == y + 1)
```

Use instead:
```r
x >= y + 1
x != y + 1
```
