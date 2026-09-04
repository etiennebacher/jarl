# deparse1
::: {.callout-note title="Added in "0.6.1" .low-opacity}
:::

## What it does

Checks for usage of `parse(deparse(x), collapse = " ")`.

## Why is this bad?

Since R 4.1.0, it is
possible to do `parse(deparse(x), collapse = " ")` with `deparse1(x)`, which is more efficient and easier
to read.

This rule comes with a safe fix but is only enabled if the project
explicitly uses R >= 4.1.0 (or if the argument `--min-r-version` is passed
with a version >= 4.1.0).

## Example

```r
paste(deparse(args(library)), collapse = " ")
```

Use instead:
```r
deparse1(args(library))
```

## References

See `?deparse1`
