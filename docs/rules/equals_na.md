# equals_na
## What it does

Check for `x == NA`, `x != NA` and `x %in% NA`, and replaces those by
`is.na()` calls.

## Why is this bad?

Comparing a value to `NA` using `==` returns `NA` in many cases:
```r
x <- c(1, 2, 3, NA)
x == NA
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
