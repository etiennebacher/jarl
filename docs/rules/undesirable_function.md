# undesirable_function

::: {.callout-note title="Added in 0.5.0" .low-opacity}
:::

## What it does

Checks for calls to functions listed as undesirable.

## Why is this bad?

Some functions should not appear in production code. For example,
`browser()` is a debugging tool that interrupts execution, and should be
removed before committing.

## Configuration options

The operation of the `undesirable_function` rule can be customised in the [configuration file](../reference/config-file.md).

Use `functions` to fully replace the default list of undesirable functions.
Use `extend-functions` to add to the default list.
Specifying both is an error.

### Default values

```toml
functions = ["browser"]
```

### TOML settings

```toml
[lint.undesirable_function]
# Replace the default list entirely:
functions = ["browser", "debug"]

# Or add to the defaults, with optional suggestions:
extend-functions = [
  { setwd = 'Use `here::here()`.' },
  "sprintf",
  { transmute = 'Use `mutate(.keep = "none")`.' },
]
```

Use a string with just the function name for the default diagnostic. Use an
inline table to attach custom suggestion text to the default message.

Names can be qualified with a package, such as `base::setwd`; qualified names
only match calls with the same package prefix.

## Example

```r
do_something <- function(abc = 1) {
   xyz <- abc + 1
   browser()      # flagged by default
   xyz
}
```
