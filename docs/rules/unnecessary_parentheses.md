# unnecessary_parentheses
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for expressions wrapped in multiple pairs of parentheses.

## Why is this bad?

Repeated parentheses do not change the meaning of the expression and can make
the code harder to read.

## Example

```r
((x + 1))
```

Use instead:

```r
x + 1
```
