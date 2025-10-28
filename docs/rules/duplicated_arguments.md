# duplicated_arguments
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
