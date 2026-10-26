# internal_function

!!! note low-opacity "Added in 0.4.0"


## What it does

Checks for usage of `:::`.

**This rule is deprecated and will be removed in a future version. Use
`undesirable_operator` instead.**

## Why is this bad?

Using `:::` to access a package's internal functions is unsafe. Those
functions are not part of the package's public interface and may be changed
or removed by the maintainers without notice. Use public functions via `::`
instead.

This rule doesn't have an automatic fix.
