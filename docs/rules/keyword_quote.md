# keyword_quote

::: {.callout-note title="Added in development version" .low-opacity}
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
