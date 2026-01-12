# sample_int
## What it does

Checks for usage of `sample(1:n, m, ...)` and replaces it with
`sample.int(n, m, ...)` for readability.

## Why is this bad?

`sample()` calls `sample.int()` internally so they have the same performance,
but the latter is more readable.

This rule is disabled by default.

## Example

```r
sample(1:10, 2)
```

Use instead:
```r
sample.int(10, 2)
```

## References

See `?sample`
