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

## R Markdown and Quarto

The chunks of an `.Rmd`/`.qmd` document run one after another in a single R
session, so a `stop()` in one chunk does make the code in the chunks after
it unreachable. This is not the case if at least one the following two options
are specified:

- `eval = FALSE` (or `#| eval: false`), because the chunk never runs at all;
- `error = TRUE` (or `#| error: true`), because the chunk prints the error
  but continues to evaluate the subsequent chunks.

Such a chunk is left out of the analysis entirely, so the following also
wouldn't be reported:

````markdown
```
#| eval: false
stop("a")
1 + 1 # unreachable but not reported
```
````

Note that unreachable code *not at the top-level* would still be reported,
e.g.:

````markdown
```
#| eval: false
f <- function() {
  stop("a")
  1 + 1 # reported as unreachable
}
```
````

A chunk whose evaluation is decided at render time (e.g. `#| eval: run_it`)
is considered evaluated.

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
