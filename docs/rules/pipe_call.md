# pipe_call

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Reports bare function names on the right-hand side of magrittr pipes.

## Why is this bad?

An explicit function call makes each step of a pipe easier to read.

This rule is disabled by default.

## Example

```r
x %>% sum
```

Use instead:

```r
x %>% sum()
```
