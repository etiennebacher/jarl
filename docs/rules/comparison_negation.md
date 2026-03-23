# comparison_negation
::: {.callout-note title="Added in 0.0.23" .low-opacity}
:::

## What it does

Checks for patterns similar to `!(... < ...)`.

## Why is this bad?

This pattern may be hard to read and could be simplified by removing the `!`
operator and inverting the operator (e.g. `<` would become `>=`).

This rule has a safe fix.

## Example

```r
!(x < y + 1)
!(x == y + 1)
```

Use instead:
```r
x >= y + 1
x != y + 1
```
