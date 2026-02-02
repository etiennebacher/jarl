# unreachable_code
## What it does

Detects code that can never be executed because it appears after control
flow statements like `return`, `break`, or `next`, or in branches that
cannot be reached.

## Why is this bad?

Unreachable code indicates a logic error or dead code that should be removed.
It clutters the codebase, confuses readers, and may indicate unintended behavior.

## Examples

```r
if (x > 5) {
  stop("hi")
} else {
  stop("bye")
}
1 + 1 # unreachable
```

```r
foo <- function(x) {
  return(x + 1)
  print("hi")  # unreachable
}
```

```r
foo <- function(x) {
  for (i in 1:10) {
    x <- x + 1
    if (x > 10) {
       break
       print("x is greater than 10") # unreachable
    }
  }
}
```
