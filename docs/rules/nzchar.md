# nzchar

::: {.callout-note title="Added in 0.5.0" .low-opacity}
:::

## What it does

Checks for usage of `x != ""` or `x == ""`, or comparisons of `nchar(x)`
with zero, such as `nchar(x) == 0`, instead of `nzchar(x)` or `!nzchar(x)`.

## Why is this bad?

`x == ""` is less efficient than `!nzchar(x)`
when x is a large vector of long strings.

One crucial difference is in the default handling of `NA_character_`,
i.e., missing strings. `nzchar(NA_character_)` is TRUE,
while `NA_character_ == ""` is NA.
Generated fixes use `nzchar(x, keepNA = TRUE)` to preserve missing values.

This rule comes with an unsafe fix because `nzchar()` can still differ for
factors or classed objects and does not preserve attributes such as names
and dimensions.

## Example

```r
x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
x[x == ""]
```

Use instead:
```r
x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
x[!nzchar(x, keepNA = TRUE)]
```

## References

See `?nzchar`
