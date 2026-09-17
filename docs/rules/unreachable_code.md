# unreachable_code
::: {.callout-note title="Added in 0.4.0" .low-opacity}
:::

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

## R Markdown and Quarto

The chunks of an `.Rmd`/`.qmd` document run one after another in a single R
session, so a `stop()` in one chunk does make the code in the chunks after
it unreachable. Two chunk options break that chain, and nothing after a
chunk carrying one is reported:

- `eval = FALSE` (or `#| eval: false`), because the chunk never runs at all;
- `error = TRUE` (or `#| error: true`), because knitr prints the condition
  and carries on rendering — through the rest of that chunk as well as the
  rest of the document.

An option whose value is decided at render time (`eval = run_it`) is read as
the ordinary case, so the document keeps stopping.
