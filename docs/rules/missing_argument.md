# missing_argument

::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for empty arguments in function calls, e.g. `paste("a", , "b")`.

## Why is this bad?

An empty argument left between commas is often a typo: a value was either
deleted by mistake or never filled in. Depending on the function it can lead
to an error or to a silently wrong result.

Several functions (e.g. `mutate()` in the `tidyverse` ecosystem) allow
trailing commas. Those are ignored by default but you can also tweak this
list of ignored functions in `jarl.toml`:

```ignore
...
[lint.missing_argument]
extend-skipped-functions = ["my_function"]
```

See the [rule-specific arguments](https://jarl.etiennebacher.com/reference/config-file#rule-specific-arguments)
for more information.

This rule has no automatic fix.

## Example

```r
paste("a", , "b")
mean(x, )
```

Use instead:
```r
paste("a", "b")
mean(x)
```
(or add additional arguments).

---

## Configuration options

The operation of `missing_argument` can be customised in the [configuration file](../reference/config-file.md).

Use `skipped-functions` to fully replace the default list of functions that are
allowed to contain missing arguments. Use `extend-skipped-functions` to add to
the default list. Specifying both is an error.

Function names in `skipped-functions` or `extend-skipped-functions` also match
namespaced calls, e.g. `skipped-functions = ["list2"]` will ignore `list2()` and
`rlang::list2()`.

Default: `skipped-functions = [
    "switch",
    "tibble",
    "list2",
    "mutate",
    "summarize",
    "transmute",
]`

```toml
[lint]
...

[lint.missing_argument]
# Ignore missing arguments in `my_function()` only.
skipped-functions = ["my_function"]
```
