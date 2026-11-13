# consecutive_signs

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Reports two or more consecutive `+` or `-` operators in code.

## Why is this bad?

R accepts expressions such as `x + + - y` and `--x`, but consecutive signs can
be accidental and make the intended operation harder to read.

No automatic fix is provided because the intended operation is ambiguous,
and arithmetic operators can dispatch to custom methods.

## Example

```r
x + - y
--x
```

Either 1) remove the unnecessary sign, 2) make the grouping more explicit
using parenthesis, or 3) add a comment explaining why this is needed.

```r
x - y
-(-x)
```
