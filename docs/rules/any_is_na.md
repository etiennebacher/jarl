# any_is_na
::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

## What it does

Checks for usage of `any(is.na(...))`, `NA %in% x`, and `NA %notin% x`.

## Why is this bad?

While both cases are valid R code, the base R function `anyNA()` is more
efficient (both in speed and memory used).

## Example

```r
x <- c(1:10000, NA)
any(is.na(x))
NA %in% x
NA %notin% x
```

Use instead:
```r
x <- c(1:10000, NA)
anyNA(x)
!anyNA(x)
```

## References

See `?anyNA`
