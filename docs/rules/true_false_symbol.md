# true_false_symbol

::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

## What it does

Checks for usage of `T` and `F` symbols. If they correspond to the `TRUE`
and `FALSE` values, then replace them by that. If they correspond to
something else, such as an object or a variable name, then no automatic
fixes are applied.

## Why is this bad?

`T` and `F` are not reserved symbols (like `break`) and therefore can be
used as variable names. Therefore, it is better for readability to replace
them by `TRUE` and `FALSE`.

It is also recommended to rename objects or parameters named `F` and `T` to
avoid confusion.

## Example

```r
x <- T
y <- F
```

Use instead:
```r
x <- TRUE
y <- FALSE
```

---

## Configuration options

The operation of `true_false_symbol` can be customised in the [configuration file](../reference/config-file.md).

Use `skipped-functions` to list functions whose arguments are allowed to contain
the `T` and `F` symbols.

Function names in `skipped-functions` also match namespaced calls, e.g.
`skipped-functions = ["foo"]` will ignore `foo(T)` and `pkg::foo(T)`.

Default: `skipped-functions = []`

```toml
[lint]
...

[lint.true_false_symbol]
skipped-functions = ["foo"]
```
