# undesirable_operator

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Checks for use of banned operators.

## Why is this bad?

Some operators may be undesirable because they make the code less robust,
more complex, or more prone to footguns. For example, `:::` accesses a package's
internal functions, meaning that they may disappear or behave differently without
notice if the package changes. `<<-` and `->>` assign outside the current environment
and may lead to code that is harder to predict.


## Configuration options

The operation of the `undesirable_operator` rule can be customised in the [configuration file](../reference/config-file.md).

Use `operators` to fully replace the default list of undesirable operators.
Use `extend-operators` to add to the default list.
Specifying both is an error.

### Default values

```toml
operators = ["->>", ":::", "<<-"]
```

### TOML settings

```toml
[lint.undesirable_operator]
# Replace the default list entirely:
operators = [":::", "%in%"]

# Or add to the defaults:
extend-operators = ["%in%"]
```

## Example

```r
package:::internal_function()  # flagged by default
value <<- 1                    # flagged by default
```
