# cyclomatic_complexity
::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Measures the cyclomatic complexity of each function, and of the top-level
code of a file, and reports the ones above `max-complexity` (15 by default).

The score starts at 1 and gains a point for every place the code can take a
different path:

* each `if` (an `else if` is an `if` too, but a bare `else` adds nothing);
* each `for`, `while` and `repeat` loop;
* each `&&` and `||`. The vectorized `&` and `|` always evaluate both sides,
  so they add nothing;
* each `switch()` arm after the first;
* each handler of `tryCatch()` and `withCallingHandlers()`, and each `try()`;
* each `return()` or `stop()` that isn't the value the function ends on.

Nested functions are scored on their own and don't count towards the
function that contains them.

## Why is this bad?

The score counts the paths through the code, which is also the number of
tests needed to cover it. A function with a high score is hard to read, hard
to test exhaustively, and hard to change without breaking one of the paths
nobody had in mind.

## Example

```r
summarize <- function(x, na.rm, kind) {
  if (!is.numeric(x)) stop("`x` must be numeric.")
  if (na.rm && anyNA(x)) x <- x[!is.na(x)]
  switch(kind,
    mean = mean(x),
    median = median(x),
    stop("Unknown `kind`.")
  )
}
```

Use instead:
```r
check_input <- function(x) {
  if (!is.numeric(x)) stop("`x` must be numeric.")
}

summarize <- function(x, na.rm, kind) {
  check_input(x)
  if (na.rm && anyNA(x)) x <- x[!is.na(x)]
  summary_fun(kind)(x)
}
```

## Options

* `max-complexity`
