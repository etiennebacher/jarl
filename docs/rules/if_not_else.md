# if_not_else

::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for `if` - `else` statements and `ifelse()` / `dplyr::if_else()` /
`data.table::fifelse()` calls whose condition is a simple negation, e.g.
`if (!cond) x else y` or `ifelse(!cond, x, y)`.

## Why is this bad?

Negating the condition forces the reader to mentally flip the branches. It is
usually clearer to write the condition positively and swap the branches:
`if (A) y else x` instead of `if (!A) x else y`.

Negated calls such as `is.null()`, `is.na()` and `missing()` are common and
read naturally, so they are allowed by default. Use the `skipped-functions`
option to change this list.

This rule does not have an automatic fix.

## Example

```r
if (!cond) x else y

ifelse(!cond, x, y)
```

Use instead:

```r
if (cond) y else x

ifelse(cond, y, x)
```

---

## Configuration options

The operation of `if_not_else` can be customised in the [configuration file](../reference/config-file.md).

Use `skipped-functions` to fully replace the default list of functions whose
negated calls are allowed as an `if`/`ifelse()` condition (e.g. `!is.null(x)`).
Use `extend-skipped-functions` to add to the default list. Specifying both is an
error.

Function names in `skipped-functions` or `extend-skipped-functions` also match
namespaced calls, e.g. `skipped-functions = ["is.null"]` will allow `is.null()`
and `base::is.null()`.

Default: `skipped-functions = ["is.null", "is.na", "missing"]`

```toml
[lint]
...

[lint.if_not_else]
# Also allow a negated `is.data.frame()` call in the condition.
extend-skipped-functions = ["is.data.frame"]
``` 
