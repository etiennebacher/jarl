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

## Configuration

By default, only `->>`, `:::`, and `<<-` are flagged. You can customize the
list in `jarl.toml`:

To replace the default list entirely:

```toml
[lint.undesirable_operator]
operators = ["%notin%", "&&"]
```

To add to the defaults:

```toml
[lint.undesirable_operator]
extend-operators = ["&&", "%in%"]
```

Specifying both `operators` and `extend-operators` is an error.

## Example

```r
package:::internal_function()  # flagged by default
value <<- 1                    # flagged by default
```
