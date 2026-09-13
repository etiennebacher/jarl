# positional_arguments
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Reports function calls that use more than a configurable number of positional
(unnamed) arguments.

## Why is this bad?

Relying on argument position forces the reader to remember the function's
signature to understand what each value means, and makes calls fragile to
changes in the argument order. Naming the arguments documents intent at the
call site and is more robust.

The maximum number of allowed positional arguments is 2 by default and can
be customized with the `max-positional-args` option in `jarl.toml` (see [rule-specific arguments](https://jarl.etiennebacher.com/reference/config-file#rule-specific-arguments)).

## Example

```r
grepl("a", x, TRUE)
```

Use instead:

```r
grepl("a", x, ignore.case = TRUE)
```
