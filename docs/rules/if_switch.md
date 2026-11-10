# if_switch

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for chains of `if`/`else if` statements that all compare the same
expression to a string with `==`, e.g.
`if (x == "a") 1 else if (x == "b") 2 else if (x == "c") 3`.

A chain is only reported if it has at least `max-branches` comparisons (4
by default), if every condition is a `==` comparison of the same expression
with a non-empty string, and if it isn't itself part of a longer chain.

## Why is this bad?

`switch()` is clearer than repeated `if`/`else` tests on the same value. It
is also faster: its evaluation time doesn't depend on the number of
branches, while an `if`/`else` chain evaluates each condition in turn.

The fix wraps the compared expression in `as.character()`. Without it,
`switch()` would select numbers and logicals by position, not by name:
`switch(3, a = "x", b = "y", c = "z")` returns `"z"` (the third branch),
whereas `3 == "c"` is `FALSE`. Factors would likewise be matched by their
integer codes instead of their labels.

This rule has an unsafe automatic fix because `NA` values make an `if` chain
error, but `switch()` silently falls back to the default (or returns an
invisible `NULL`). `NA_character_` even matches a branch named `"NA"`.

Empty strings are not reported because `switch()` cannot have an empty
string as an argument name.

## Configuration options

The operation of the `if_switch` rule can be customised in the [configuration file](../reference/config-file.md).

Use `max-branches` to set the number of equality branches (`if` and
`else if` conditions) from which an `if`/`else` chain is reported. It must
be at least 2.

### Default values

```toml
max-branches = 4
```

### TOML settings

```toml
[lint]
...

[lint.if_switch]
max-branches = 5
```

## Examples

```r
if (x == "a") {
  1
} else if (x == "b") {
  2
} else if (x == "c") {
  3
}
```

Use instead:
```r
switch(as.character(x), a = 1, b = 2, c = 3)
```

## References

See `?switch`.
