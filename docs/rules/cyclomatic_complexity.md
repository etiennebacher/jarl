# cyclomatic_complexity

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Measures the cyclomatic complexity of each function, and of each top-level
expression of a file, and reports the ones above `max-complexity` (20 by
default).

The score is the one computed by the
[cyclocomp](https://cran.r-project.org/package=cyclocomp) package, which is
also the one used by `lintr::cyclocomp_linter()`: it is the number of
edges minus the number of nodes plus 2 of the control-flow graph of the
code. In practice, `if`, `for`, `while`, `repeat`, `break`, `next`, `&&`,
`||` and `return()` (unless it is the last expression) add paths. `stop()`,
`tryCatch()` and `try()` do not.

There are a few differences with cyclocomp, where it gives a score that
doesn't match the number of paths through the code:

* a chain of `&&` or `||` adds one path per operator. cyclocomp scores
  `if (a || b || c || d) stop()` as 11, this rule as 5;
* `switch()` adds one path per arm, while cyclocomp ignores it;
* a function defined inside another one is scored on its own and counts as
  a single node in the function that contains it, while cyclocomp adds all
  of its paths to the ones of the outer function.

## Why is this bad?

The score counts the paths through the code, which is also the number of
tests needed to cover it. A function with a high score is hard to read,
hard to test exhaustively, and hard to change without breaking one of the
paths nobody had in mind.

## Configuration options

The operation of the `cyclomatic_complexity` rule can be customised in the [configuration file](../reference/config-file.md).

Use `max-complexity` to set the highest score an expression is allowed to
reach before it is reported. It must be at least 1.

### Default values

```toml
max-complexity = 20
```

### TOML settings

```toml
[lint]
...

[lint.cyclomatic_complexity]
# Only report the expressions that are really tangled.
max-complexity = 25
```

## Example

```r
summarize <- function(x, na.rm, kind) {
  if (!is.numeric(x)) stop("`x` must be numeric.")
  if (na.rm && anyNA(x)) x <- x[!is.na(x)]
  for (i in seq_along(x)) {
    if (x[i] < 0) next
    if (kind == "mean") return(mean(x))
  }
  median(x)
}
```

Use instead: split the function in smaller ones, each with a few branches.

## Options

* `max-complexity`
