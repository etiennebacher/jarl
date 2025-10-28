# any_duplicated
## What it does

Checks for usage of `any(duplicated(...))`.

## Why is this bad?

`any(duplicated(...))` is valid code but requires the evaluation of
`duplicated()` on the entire input first.

There is a more efficient function in base R called `anyDuplicated()` that
is more efficient, both in speed and memory used. `anyDuplicated()` returns
the index of the first duplicated value, or 0 if there is none.

Therefore, we can replace `any(duplicated(...))` by `anyDuplicated(...) > 0`.

## Example

```r
x <- c(1:10000, 1, NA)
any(duplicated(x))
```

Use instead:
```r
x <- c(1:10000, 1, NA)
anyDuplicated(x) > 0
```

## References

See `?anyDuplicated`
