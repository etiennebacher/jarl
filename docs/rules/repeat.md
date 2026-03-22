# repeat
::: {.callout-note title="Added in 0.0.19" .low-opacity}

## What it does

Checks use of `while (TRUE)` and recommends the use of `repeat` instead.

## Why is this bad?

`while (TRUE)` is valid R code but `repeat` better expresses the intent of
infinite loop.

## Example

```r
while (TRUE) {
  # ...
  break
}
```

Use instead:
```r
repeat {
  # ...
  break
}
```
