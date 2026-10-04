# function_argument

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Checks for arguments without defaults that appear after arguments with
defaults in function definitions. The `...` argument is ignored.

## Why is this bad?

Placing required arguments before optional arguments makes functions easier
to understand and call.

This rule has no automatic fix because changing the argument order can
break existing calls.

This rule is disabled by default.

## Example

```r
function(x = 1, y) {
  x + y
}
```

Use instead:

```r
function(y, x = 1) {
  x + y
}
```
