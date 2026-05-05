# equals_na
::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

## What it does

Check for `x == NA`, `x != NA`, `x %in% NA` and `x %notin% NA`, and
replaces those by `is.na()` or `!is.na()` calls.

## Why is this bad?

Comparing a value to `NA` using `==` returns `NA` in many cases:
```r
x <- c(1, 2, 3, NA)
x == NA
#> [1] NA NA NA NA
```
which is very likely not the expected output.

## Example

```r
x <- c(1, 2, 3, NA)
x == NA
```

Use instead:
```r
x <- c(1, 2, 3, NA)
is.na(x)
```
