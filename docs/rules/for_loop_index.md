# for_loop_index

::: {.callout-note title="Added in 0.0.16" .low-opacity}
:::

## What it does

Checks whether the index variable in a `for` loop also occurs in its
sequence expression, including inside calls, parentheses, and arithmetic.
Argument labels, field and namespace names, subscript expressions, and
function definitions are ignored.

## Why is this bad?

`for (x in x)` or `for (x in foo(x))` are confusing to read and can lead
to errors.

## Example

```r
x <- c(1, 2, 3)
for (x in x) {
  x + 1
}
```

Use instead:
```r
x <- c(1, 2, 3)
for (i in x) {
  i + 1
}
```
