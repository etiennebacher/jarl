# which_grepl
::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

## What it does

Checks for usage of `which(grepl(...))` and replaces it with `grep(...)`.

## Why is this bad?

`which(grepl(...))` is harder to read and is less efficient than `grep()`
since it requires two passes on the vector.

This rule has an automatic fix for direct calls where `which()` only
contains the `grepl()` call, and for pipe chains where the final `which()`
has no arguments and the piped value can be unambiguously assigned to
`grepl()`'s `pattern` or `x` argument.

Calls with additional arguments to `which()` are reported but not fixed
because those arguments cannot generally be preserved by replacing
`which()` with `grep()`. The exception is a literal `arr.ind = TRUE` or
`FALSE`: `grepl()` returns a vector without dimensions, so `arr.ind` has no
effect.

## Example

```r
x <- c("hello", "there")
which(grepl("hell", x))
which(grepl("foo", x))
```

Use instead:
```r
grep("hell", x)
grep("foo", x)
```

## References

See `?grep`
