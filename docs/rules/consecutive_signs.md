# consecutive_signs

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Reports two or more consecutive `+` or `-` operators in code. Comments and
string contents are not checked. Whitespace and comments between operators
do not break a sequence, but parentheses do.

This rule is disabled by default. Enable it with
`--select consecutive_signs` or by selecting the `SUSP` category.

## Why is this bad?

R accepts expressions such as `x + + - y` and `--x`, but consecutive signs can
be accidental and make the intended operation harder to read.

Some domain-specific languages intentionally use consecutive signs, such as
`igraph::graph_from_literal(A--B)`. Suppress the rule for intentional uses.

No automatic fix is provided because the intended operation is ambiguous,
and arithmetic operators can dispatch to custom methods.

## Example

```r
x + + - y
--x
```

If intentional, make the grouping explicit:

```r
x + (+y)
x - (-y)
```
