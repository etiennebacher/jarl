# any_is_na
## What it does

Checks for usage of `any(is.na(...))` and `NA %in% x`.

## Why is this bad?

While both cases are valid R code, the base R function `anyNA()` is more
efficient (both in speed and memory used).

## Example

```r
x <- c(1:10000, NA)
any(is.na(x))
NA %in% x
```

Use instead:
```r
x <- c(1:10000, NA)
anyNA(x)
```

## References

See `?anyNA`
