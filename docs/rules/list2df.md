# list2df
## What it does

Checks for usage of `do.call(cbind.data.frame, x)`.

## Why is this bad?

The goal of `do.call(cbind.data.frame, x)` is to concatenate multiple lists
elements of the same length into a `data.frame`. Since R 4.0.0, it is
possible to do this with `list2DF(x)`, which is more efficient and easier
to read than `do.call(cbind.data.frame, x)`.

This rule comes with a safe fix but is only enabled if the project
explicitly uses R >= 4.0.0 (or if the argument `--min-r-version` is passed
with a version >= 4.0.0).

## Example

```r
x <- list(a = 1:10, b = 11:20)
do.call(cbind.data.frame, x)
```

Use instead:
```r
x <- list(a = 1:10, b = 11:20)
list2DF(x)
```

## References

See `?list2DF`
