# which_grepl
::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

## What it does

Checks for usage of `which(grepl(...))` and replaces it with `grep(...)`.

## Why is this bad?

`which(grepl(...))` is harder to read and is less efficient than `grep()`
since it requires two passes on the vector.

## Example

```r
which(grepl("foo", x))
```

Use instead:
```r
grep("foo", x)
```

## References

See `?grep`
