# coalesce
## What it does

Checks for usage of `if (is.null(x)) y else x` or
`if (!is.null(x)) x else y` and recommends using `x %||% y` instead.

## Why is this bad?

Using the coalesce operator `%||%` is more concise and readable than
an if-else statement checking for null.

This rule is only enabled if the project uses R >= 4.4.0, since `%||%` was
introduced in this version.

This rule contains some automatic fixes, but only for cases where the
branches are on a single line. For instance,
```r,ignore
if (is.null(x)) {
  y
} else {
  x
}
```
would be simplified to `x %||% y`, but
```r,ignore
if (is.null(x)) {
  y <- 1
  y
} else {
  x
}
```
wouldn't.

## Example

```r
x <- 1
y <- 2

if (is.null(x)) y else x

if (!is.null(x)) {
  x
} else {
  y
}
```

Use instead:
```r
x <- 1
y <- 2

x %||% y # (in both cases)
```

## Reference

See `?Control`
