# internal_function
## What it does

Checks for usage of `:::`.

## Why is this bad?

Using `:::` to access a package's internal functions is unsafe. Those
functions are not part of the package's public interface and may be changed
or removed by the maintainers without notice. Use public functions via `::`
instead.

This rule doesn't have an automatic fix.
