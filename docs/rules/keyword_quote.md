# keyword_quote

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for unnecessary quoting of:

* argument names in function calls, e.g. `c("a" = 1)`;
* targets of assignment, e.g. `"foo" <- 1`;
* targets of extraction with `$` or `@`, e.g. `x$"foo"`.

It also reports non-syntactic assignment and extraction targets that use
quotes instead of backticks, e.g. `"foo bar" <- 1`.

## Why is this bad?

Quotes and backticks are only needed when a name is not syntactic, i.e.
when it contains special characters (such as spaces) or is a reserved word
(such as `if` or `TRUE`). In other cases, they add visual noise.

When quoting is needed for an assignment or extraction target, backticks
should be used: they are the standard way to write non-syntactic names in R,
whereas the fact that strings also work there is an exception.

## Example

```r
x <- c("a" = 1, `b` = 2)
"foo" <- 1
"foo bar" <- 2
x$"bar"
```

Use instead:
```r
x <- c(a = 1, b = 2)
foo <- 1
`foo bar` <- 2
x$bar
```

## References

See `?make.names`, `?Reserved` and `?Quotes`.
