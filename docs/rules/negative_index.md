# negative_index

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for unary minus in `[]` indices, such as `x[-idx]`, except for
directly negated literals such as `x[-1]`.
A `c()` call is also treated as literal when all its arguments are literals.
Negation inside logical expressions or calls to `between()`, `which()`, and
`order()` is ignored, as in `x[x > -1]` and `x[order(-x)]`.

## Why is this bad?

If `idx` is empty, `x[-idx]` selects no elements instead of keeping all of
`x`. This can happen with `x[-which(condition)]` when nothing matches.

This rule also reports expressions such as `x[-1 * idx]`, `x[c(-1, -idx)]`,
and `x[-2 * 3]`. It does not determine whether an index is actually empty or
whether a surrounding condition already checks its length.

No automatic fix is provided. Handle empty indices explicitly or use a
logical index when possible.

## Example

```r
x <- 1:10
x[-which(x == 99)]
```

Use instead:

```r
x <- 1:10
x[x != 99]
```
