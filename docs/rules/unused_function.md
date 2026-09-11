# unused_function

::: {.callout-note title="Added in 0.5.0" .low-opacity}
:::

## What it does

Checks for unused functions in R packages. It looks for:

- Functions defined in `R/` that are not exported and not used anywhere in
  the package (including `R/`, `inst/tinytest/`, `inst/tests/`, `src/`, and
  `tests/`).
- Functions defined in `tests/` that are not used anywhere in `tests/`.
- Functions defined in `inst/tinytest/` or `inst/tests/` that are not used
  anywhere within that directory.

## Why is this bad?

Functions must be documented in NAMESPACE to be exported to end users. A
function that is never called nor exported is likely dead code left over
from refactoring. Removing unused internal functions keeps the codebase
easier to understand and maintain.

## Limitations

There are many ways to call a function in R code (e.g. `foo()`,
`do.call("foo", ...)`, `lapply(x, foo)` among others). Jarl tries to limit
false positives as much as possible, at the expense of false negatives. This
means that reporting a function that is actually used somewhere (false positive)
is considered a bug, but not reporting a function that isn't used anywhere
(false negative) isn't considered a bug (but can be suggested as a feature
request).

## Example

```r
# In NAMESPACE: export(public_fn)

# In R/public.R:
public_fn <- function(x) {
  check_character(x)
}

# In R/helper.R:
check_character <- function(x) {
  stopifnot(is.character(x))
}
check_length <- function(x, y) {
  stopifnot(length(x) == y)
}

# `public_fn()` is exported by the package, so it is considered used.
# `check_character()` isn't exported but used in `public_fn`.
# `check_length()` isn't exported but and isn't used anywhere, so it is
# reported.
```

---

## Configuration options

The operation of `unused_function` can be customised in the [configuration file](../reference/config-file.md).

Use `skipped-functions` to fully replace the default list of functions that are
allowed to be unused in the R package. Function names in `skipped-functions`
**are parsed as regular expressions** (this differs from other rules that have a
`skipped-functions` argument).

`unused_function` might return false positives because Jarl cannot statically
determine whether a function is used. By default, Jarl will hide `unused_function`
diagnostics if there are more than 50, as this would suggest that the package
has some internal mechanism to use those functions. This number can be changed
with the `threshold-ignore` argument.

Defaults:

- `skipped-functions = []`
- `threshold-ignore = 50`

```toml
[lint]
...

[lint.unused_function]
# Ignore all functions that start with "pl_" or "cs_", and the function
# "my.function"
skipped-functions = ["^cs_", "^pl_", "my\\.function"]
# Set a custom threshold above which diagnostics for this rule aren't reported
# (this is basically equivalent to never hiding unused functions).
threshold-ignore = 10000
```
