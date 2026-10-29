# function_return

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Checks that functions consistently use implicit or explicit `return()`. The
preferred mode can be configured, see below.

The returned values are searched in the last expression of the function
body, in the branches of terminal `if` statements and in the arms of
terminal `switch()` calls.

When `mode = "explicit"`, only functions whose body spans several lines
are checked. Functions ending with a call that stops the execution (such as
`stop()` or `UseMethod()`) and package hooks (such as `.onLoad()`) are not
reported.

## Why is this bad?

In R, a function automatically returns the value of its last expression,
so a terminal `return()` call is redundant. At the same time, explicit
`return()` can clarify code, for instance across `if` branches. Using
explicit or implicit `return()` depends on personal preferences, but
mixing the two styles decreases readability and consistency.

This rule has an unsafe automatic fix because `return` may refer to a
function argument instead of `base::return()`, for instance in
`function(x, return) x`. Adding a call to `return()` in this function
would fail if the argument `return` is not provided.

The automatic fix is not available when the last expression is an assignment
or a loop, since wrapping it in `return()` would change its meaning or its
visibility.

## Configuration options

The operation of the `function_return` rule can be customised in the [configuration file](../reference/config-file.md).

The `mode` option takes a single value (`"implicit"` or `"explicit"`)
indicating the preferred return style in functions. If `mode = "implicit"`,
then any `return()` call in a terminal position of a function will be
reported. If `mode = "explicit"`, then any terminal expression that isn't
wrapped in `return()` (or in a function that stops execution, such as
`stop()`) will be reported.

### Default values

```toml
mode = "implicit"
```

### TOML settings

```toml
[lint]
...

[lint.function_return]
# Require explicit `return()` calls instead
mode = "explicit"
```

## Example

```r
f <- function(x) {
  if (x > 0) {
    return("positive")
  }
  return("negative")
}
```

If the preferred mode is "implicit", use instead:
```r
f <- function(x) {
  if (x > 0) {
    return("positive")
  }
  "negative"
}
```

## References

- [Tidyverse style guide](https://style.tidyverse.org/functions.html#return)
