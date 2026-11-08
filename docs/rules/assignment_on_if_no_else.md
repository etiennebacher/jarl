# assignment_on_if_no_else

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Reports assignments whose value is an `if` expression without a final `else`
branch, when the assignment overwrites an earlier value.

## Why is this bad?

When no branch is taken, an `if` expression without a final `else` evaluates
to `NULL`. This can be unexpected since one could believe that if no branch
is taken then the previous value is kept.

## Example

```r
df <- mtcars
# If `condition` is `FALSE`, `df` becomes `NULL` instead of
# keeping its original value (`mtcars`).
df <- if (condition) {
  mtcars[1:5, ]
}

value <- 1
# If neither `a` nor `b` is `TRUE`, `value` becomes `NULL`
# instead of keeping its original value (`1`).
value <- if (a) {
  1
} else if (b) {
  2
}
```

Use instead:

```r
if (condition) {
  df <- mtcars[1:5, ]
}

if (a) {
  value <- 1
} else if (b) {
  value <- 2
}
```

If `NULL` is the intended fallback, add a final `else` branch that says so.
