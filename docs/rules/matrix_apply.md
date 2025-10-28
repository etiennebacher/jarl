# matrix_apply
## What it does

Checks for usage of `apply(x, 1/2, mean/sum)`.

## Why is this bad?

`apply()` with `FUN = sum` or `FUN = mean` are inefficient when `MARGIN` is
1 or 2. `colSums()`, `rowSums()`, `colMeans()`, `rowMeans()` are both easier
to read and much more efficient.

This rule provides an automated fix, except when extra arguments (outside
of `na.rm`) are provided. In other words, this would be marked as lint and
could be automatically replaced:
```r
dat <- data.frame(x = 1:3, y = 4:6)
apply(dat, 1, mean, na.rm = TRUE)
```
but this wouldn't:
```r
dat <- data.frame(x = 1:3, y = 4:6)
apply(dat, 1, mean, trim = 0.2)
```

## Example

```r
dat <- data.frame(x = 1:3, y = 4:6)
apply(dat, 1, sum)
apply(dat, 2, sum)
apply(dat, 1, mean)
apply(dat, 2, mean)
apply(dat, 2, mean, na.rm = TRUE)
```

Use instead:
```r
dat <- data.frame(x = 1:3, y = 4:6)
rowSums(dat)
colSums(dat)
rowMeans(dat)
colMeans(dat)
colMeans(dat, na.rm = TRUE)
```

## References

See `?colSums`
