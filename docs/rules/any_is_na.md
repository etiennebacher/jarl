# any_is_na
## What it does

Checks for usage of `any(is.na(...))`.

## Why is this bad?

`any(is.na(...))` is valid code but requires the evaluation of `is.na()` on
the entire input first.

There is a more efficient function in base R called `anyNA()` that is more
efficient, both in speed and memory used.

## Example

```r
x <- c(1:10000, NA)
any(is.na(x))
```

Use instead:
```r
x <- c(1:10000, NA)
anyNA(x)
```

## References

See `?anyNA`
