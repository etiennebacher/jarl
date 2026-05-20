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

For 2, specifying `.open` and `.close` delimiters when the string does not
contain those delimiters means `glue()` will not perform any interpolation,
making the function call unnecessary.

Both cases do not have an automatic fix.

## Example

```r
glue("abc")
glue('{a}', .open = '<', .close = '>')
```

Use instead:
```r
"abc"
# For the second case, either use default delimiters {}, or ensure the string contains the specified delimiters
```

## References

See `?glue::glue`
