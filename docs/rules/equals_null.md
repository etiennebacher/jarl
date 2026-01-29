# equals_null
## What it does

Check for `x == NULL`, `x != NULL` and `x %in% NULL`, and replaces those by
`is.null()` calls.

## Why is this bad?

Comparing a value to `NULL` using `==` returns a `logical(0)` in many cases:
```r
x <- NULL
x == NULL
#> logical(0)
```
which is very likely not the expected output.

## Example

```r
x <- c(1, 2, 3)
y <- NULL

x == NULL
y == NULL
```

Use instead:
```r
x <- c(1, 2, 3)
y <- NULL

is.null(x)
is.null(y)
```
