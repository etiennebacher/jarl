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

---

## Configuration options

The operation of `unreachable_code` can be customised in the [configuration file](../reference/config-file.md).

Use `stopping-functions` to fully replace the default list of functions that are
considered to stop execution (never return). Use `extend-stopping-functions` to
add to the default list. Specifying both is an error.

Function names in `stopping-functions` or `extend-stopping-functions` also match
namespaced calls, e.g. `stopping-functions = ["abort"]` will consider `abort()`
and `rlang::abort()` as stopping functions.

Default: `stopping-functions = ["stop", ".Defunct", "abort", "cli_abort",
"q", "quit"]`.

```toml
[lint]
...

[lint.unreachable_code]
# Add a custom function to the list of stopping functions
extend-stopping-functions = ["my_custom_stop"]
```
