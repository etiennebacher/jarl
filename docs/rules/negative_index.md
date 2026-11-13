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
and `x[-2 * 3]`. A directly negated variable is allowed inside an `if` branch
or `while` body whose condition establishes that it is nonempty, such as
`if (length(idx))`, `if (n > 1)`, or `while (length(idx) > 0)`, with no
intervening assignment to that variable.
An earlier check that exits with `return()`, `stop()`, `next`, or `break`
when the index is empty also protects subsequent uses in the same block.
Checks such as `if (any(idx > 0))` and `stopifnot(length(idx) > 0)` are also
recognized. `stopifnot(idx > 0)` does not exclude empty indices.
A preceding numeric literal assignment, such as `idx <- 1`, also establishes
nonemptiness if the variable is unchanged before use.
Other runtime values and indirect checks are not inferred.

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
