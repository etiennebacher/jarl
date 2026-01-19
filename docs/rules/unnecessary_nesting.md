# unnecessary_nesting
## What it does

This rule detects nested `if` conditions that could be gathered into a single
one.

## Why is this bad?

Nesting `if` conditions when it is not necessary may hurt readability.

This rule has a safe fix, in the sense that it will not change the meaning
of the code. However, it may produce code that is incorrectly formatted.

## Example

```r
if (x > 0) {
  if (y > 0) {
    print("x and y are greather than 0")
  }
}
```

Use instead:

```r
if (x > 0 && y > 0) {
  print("x and y are greather than 0")
}
```
