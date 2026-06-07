# literal_coercion
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for coercing a literal to a specific type, e.g. `as.integer(1)` or
`as.character(1)`. This also covers the `rlang` functions `lgl()`, `int()`,
`dbl()` and `chr()`.

## Why is this bad?

Such a coercion is done at runtime even though the result is known
statically. Writing the literal value directly (e.g. `1L` instead of
`as.integer(1)`) is clearer and avoids the unnecessary computation.

This rule also recommends using the `NA` typed versions directly, e.g.
`NA_character_` instead of `as.character(NA)`.

## Example

```r
as.integer(1)
as.character(1)
as.double("foo")
as.logical("true")
rlang::int(1)
```

Use instead:
```r
1L
"1"
NA_real_
TRUE
1L
```
