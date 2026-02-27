# unused_function
## What it does

Checks for unused functions, currently limited to R packages. It looks for
functions defined in the `R` folder that are not exported and not used
anywhere in the package (including the `R`, `inst/tinytest`, `src`, and
`tests` folders).

## Why is this bad?

An internal function that is never called is likely dead code left over from
refactoring. Removing it keeps the codebase easier to understand and
maintain.

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
