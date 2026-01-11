# unreachable_code
## What it does

Detects code that can never be executed because it appears after control
flow statements like `return`, `break`, or `next`, or in branches that
cannot be reached.

## Why is this bad?

Unreachable code indicates a logic error or dead code that should be removed.
It clutters the codebase, confuses readers, and may indicate unintended behavior.

## Example

```r
foo <- function(x) {
  return(x + 1)
  print("This will never execute")  # unreachable
}
```

```r
for (i in 1:10) {
  break
  x <- i  # unreachable
}
```
