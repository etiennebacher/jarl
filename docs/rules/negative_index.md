# negative_index

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for unary minus in `[]` indices, such as `x[-idx]` and `x[-which(x == 99)]`.

The rule will not report for cases below:

- Literals and constant arithmetic, such as `x[-1]`, `x[-2 * 3]`, and `x[c(-1, -2)]`.
- Negation inside comparisons, such as `x[x > -n]`.
- Negation inside `which()`, `order()`, and `between()`.
- `-idx` guarded by an `if` or `while` condition proving it is nonempty,
  such as `length(idx) > 0`, `length(idx)`, or `any(idx > 0)`.
- `-idx` after a check that exits with `return()`, `stop()`, `next`, or `break`
  when the index is empty, or an assertion such as `stopifnot(length(idx) > 0)`.
- `-idx` after a constant numeric assignment, such as `idx <- 1` or `idx <- 2 * 3`.

## Why is this bad?

If `idx` is empty, `x[-idx]` selects no elements instead of keeping all of
`x`. For example, trying to remove a value that is not present returns an
empty vector:

```r
x <- 1:10
x[-which(x == 99)]
#> integer(0)
```

Since `99` is not in `x`, the intended result is the unchanged vector `1:10`
rather than an empty vector.

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
