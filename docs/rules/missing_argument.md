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
