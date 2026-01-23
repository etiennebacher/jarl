# equals_nan
## What it does

Check for `x == NaN`, `x != NaN` and `x %in% NaN`, and replaces those by
`is.nan()` calls.

## Why is this bad?

Comparing a value to `NaN` using `==` returns `NaN` in many cases:
```r
x <- c(1, 2, 3, NaN)
x == NaN
#> [1] NA NA NA NA
```
which is very likely not the expected output.

## Example

```r
x <- c(1, 2, 3, NaN)
x == NaN
```

Use instead:
```r
x <- c(1, 2, 3, NaN)
is.nan(x)
```
