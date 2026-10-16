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

Entries can be strings or inline tables mapping a function to a custom
suggestion.
Function names in inline tables must be quoted.
Names can be qualified with a package, such as `base::setwd`, in which case
they only match calls with the same package prefix.

### Default values

```toml
functions = ["browser"]
```

### TOML settings

```toml
[lint.undesirable_function]
# Replace the default list entirely:
functions = ["browser", "debug"]

# Or add to the defaults:
extend-functions = ["debug"]

# Or add to the defaults, with optional suggestions:
extend-functions = [
  { "setwd" = 'Use `here::here()`.' },
  "sprintf",  # No suggestion for this case
  { "transmute" = 'Use `mutate(.keep = "none")`.' },
]
```

## Example

```r
do_something <- function(abc = 1) {
   xyz <- abc + 1
   browser()      # flagged by default
   xyz
}
```
