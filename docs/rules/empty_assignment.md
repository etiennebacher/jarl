# empty_assignment
::: {.callout-note title="Added in 0.0.8" .low-opacity}

## What it does

Looks for patterns such as `x <- {}`.

## Why is this bad?

Assignment of `{}` is the same as assignment of `NULL`, but the latter is
clearer.

## Example

```r
a <- {}
b <- {

}
```

Use instead:
```r
a <- NULL
b <- NULL
```
