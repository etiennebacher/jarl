# unused_object
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Detects objects that are defined (i.e. assigned a value) but never used.

## Why is this bad?

Unused assignments are usually a sign of dead code or a bug. Removing them
reduces noise.

## Examples

```r
x <- 1   # unused
y <- 2
print(y)
```
