# duplicated_arguments

::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

## What it does

Checks for duplicated arguments in function calls.

## Why is this bad?

While some cases of duplicated arguments generate run-time errors (e.g.
`mean(x = 1:5, x = 2:3)`), this is not always the case (e.g.
`c(a = 1, a = 2)`).

This linter is used to discourage explicitly providing duplicate names to
objects. Duplicate-named objects are hard to work with programmatically and
should typically be avoided.

## Example

```r
list(x = 1, x = 2)
```

---

## Configuration options

The operation of `duplicated_arguments` can be customised in the [configuration file](../reference/config-file.md).

Use `skipped-functions` to fully replace the default list of functions that are
allowed to have duplicated arguments. Use `extend-skipped-functions` to add to
the default list. Specifying both is an error.

Function names in `skipped-functions` or `extend-skipped-functions` also match
namespaced calls, e.g. `skipped-functions = ["list2"]` will ignore `list2()` and
`rlang::list2()`.

Default: `skipped-functions = ["c", "mutate", "summarize", "transmute"]`

```toml
[lint]
...

[lint.duplicated_arguments]
# Ignore duplicated arguments in `list()` only.
skipped-functions = ["list"]
```
