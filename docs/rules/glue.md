# glue
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Multiple checks for `glue()`:

1. checks whether `glue()` evaluates some R code between delimiters;
2. checks whether `glue()` would error when evaluated because of incomplete
   delimiters.

## Why is this bad?

For 1, using `glue()` with only a constant string, e.g. `glue("abc")`, is
useless and less readable. You can just use the string directly.

For 2, having incomplete delimiters would error when evaluated,
so this indicates a bug.

Both cases do not have an automatic fix.

## Example

```r
glue("abc")
glue('{a}', .open = '<', .close = '>')
glue("{abc")
```

Use instead:
```r
"abc"
# For the second case, either use default delimiters {},
# or ensure the string contains the specified delimiters
# For the third case, fix the string to have complete delimiters,
# e.g. glue("{abc}")
```

## References

See `?glue::glue`
