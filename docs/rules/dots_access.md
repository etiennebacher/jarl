# dots_access

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for `names(list(...))`, `length(list(...))`, and `list(...)[[n]]`
where `...` is the function's variadic arguments.

## Why is this bad?

`...names()`, `...length()`, and `...elt(n)` access `...` directly without
constructing a list. For positive integer literal indices, `..1`, `..2`,
and so on are more concise than `...elt(1)`, `...elt(2)`, and so on.
Character literal indices and additional list elements are not checked.

This rule provides an automatic fix and only applies to projects supporting
R >= 4.1.0, when `...names()` became available.
The replacements do not evaluate unselected arguments, so side effects and
errors in those arguments will no longer occur. Element access assumes a
single numeric index; character and recursive indices need `list(...)`.

## Example

```r
f <- function(...) {
  names(list(...))
  length(list(...))
  list(...)[[2L]]
  list(...)[[i + 1]]
}
```

Use instead:
```r
f <- function(...) {
  ...names()
  ...length()
  ..2
  ...elt(i + 1)
}
```
