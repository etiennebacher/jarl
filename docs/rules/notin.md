# notin

::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for usage of `!(x %in% y)` and recommends using `%notin%` instead.

## Why is this bad?

Starting from R 4.6.0, the `%notin%` operator is available in base R.
Using `%notin%` makes the intent clearer than wrapping `%in%` in a negation.

## Example

```r
if (!(x %in% choices)) {
  print("x is not in choices")
}
```

Use instead:

```r
if (x %notin% choices) {
  print("x is not in choices")
}
```

## References

See `?match`
