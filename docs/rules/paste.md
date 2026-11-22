# paste

::: {.callout-note title="Added in development version" .low-opacity}
:::

## What it does

Checks for calls to `paste()` and `paste0()` that can be replaced by a
simpler function:

1. `paste(..., sep = "")` can be replaced by `paste0(...)`;
2. `paste(x, collapse = ", ")` can be replaced by `toString(x)`;
3. `paste(rep(x, times), collapse = "")` can be replaced by
   `strrep(x, times)`.

The same applies to `paste0()` for the last two cases.

`paste(..., sep = "")` inside `expression()` is not reported: there,
`paste()` is the plotmath function that juxtaposes its arguments (see
`?plotmath`).

## Why is this bad?

`paste0()`, `toString()` and `strrep()` are dedicated functions for these
cases. They are shorter and make the intent clearer.

This rule has an automatic fix.

## Example

```r
paste("a", "b", sep = "")
paste(x, collapse = ", ")
paste0(rep("*", 10), collapse = "")
```

Use instead:
```r
paste0("a", "b")
toString(x)
strrep("*", 10)
```

## References

See `?paste`, `?toString`, `?strrep` and `?plotmath`
