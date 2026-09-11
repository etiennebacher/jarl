# implicit_assignment

::: {.callout-note title="Added in 0.0.17" .low-opacity}
:::

## What it does

Checks for implicit assignment in function calls and other situations.

## Why is this bad?

Assigning inside function calls or other situations such as in `if()` makes
the code difficult to read, and should be avoided.

## Example

```r
mean(x <- c(1, 2, 3))
x

if (any(y <- x > 0)) {
  print(y)
}
```

Use instead:
```r
x <- c(1, 2, 3)
mean(x)
x

larger <- x > 0
if (any(larger)) {
  print(larger)
}
```

## References

See:

- [https://style.tidyverse.org/syntax.html#assignment](https://style.tidyverse.org/syntax.html#assignment)

---

## Configuration options

The operation of `implicit_assignment` can be customised in the [configuration file](../reference/config-file.md).

Use `skipped-functions` to fully replace the default list of functions that are
allowed to contain implicit assignment. Use `extend-skipped-functions` to add to
the default list. Specifying both is an error.

Function names in `skipped-functions` or `extend-skipped-functions` also match
namespaced calls, e.g. `skipped-functions = ["list2"]` will ignore `list2()` and
`rlang::list2()`.

Default: `skipped-functions = ["alist", "expect_error", "expect_warning", "expect_message",
"expect_silent", "expect_defunct", "expect_deprecated", "expect_snapshot",
"expect_no_condition", "expect_no_warning", "expect_no_error", "expect_no_message",
"quote", "suppressMessages", "suppressWarnings", "try"]`
(`expect_`functions come from the `testthat` package, except `expect_defunct` and
`expect_deprecated` which come from the `lifecycle` package)

```toml
[lint]
...

[lint.implicit_assignment]
# Ignore implicit assignment in `list()` only.
skipped-functions = ["list"]
```
