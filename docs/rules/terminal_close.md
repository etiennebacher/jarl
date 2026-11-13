# terminal_close

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks for function definitions whose last call is `close(x)` or
`return(close(x))`.

## Why is this bad?

`close()` is used to close a connection but might never trigger if there is
an error before it is called. It is safer to call `on.exit(close(x))` right
after the connection is created, so that it is closed however the function
exits.

This rule doesn't have an automatic fix because `on.exit()` must be placed
next to the place where the connection is opened.

## Example

```r
f <- function(path) {
  conn <- file(path, open = "r")
  out <- readLines(conn)
  close(conn)
}
```

Use instead:
```r
f <- function(path) {
  conn <- file(path, open = "r")
  on.exit(close(conn))
  readLines(conn)
}
```

## References

See `?on.exit` and `?close`.
