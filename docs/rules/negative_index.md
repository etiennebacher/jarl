# negative_index

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `[]` indices whose outermost operator is unary minus, such as
`x[-idx]` and `x[-which(x == 99)]`. Parentheses around the index are ignored.
A minus inside a call or multiplication, as in `x[head(idx, -1)]` or
`x[-1 * idx]`, is outside the scope of this rule.

The rule will not report for cases below:

- Literals and constant arithmetic, such as `x[-1]`, `x[-(2 * 3)]`, and `x[-c(1, 2)]`.
- A single index such as `x[-length(x)]`, where the same variable is subsetted
  and passed to `length()`. If `x` is empty, selecting nothing is expected.
- `-idx` guarded by an `if` or `while` condition proving it is nonempty,
  such as `length(idx) > 0`, `length(idx)`, or `any(idx > 0)`.
- `-idx` after a check that exits when the index is empty, for example with
  `return()`, `stop()`, `rlang::abort()`, `cli::cli_abort()`, `next`, or `break`,
  or after an assertion such as `stopifnot(length(idx) > 0)`.
- `-idx` after a constant numeric assignment, such as `idx <- 1` or `idx <- 2 * 3`.
  Assigning a zero literal still triggers a warning on `x[-idx]` because `x[-0]`
  selects nothing.

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
