# redundant_ifelse
## What it does

This checks for cases of `ifelse()`, `dplyr::if_else()`, and
`data.table::fifelse()` where the output is always a boolean. In those cases,
using the condition directly is enough, the function call is redundant.

## Why is this bad?

This rule looks for 4 cases:

- `ifelse(condition, TRUE, FALSE)`
- `ifelse(condition, FALSE, TRUE)`
- `ifelse(condition, TRUE, TRUE)`
- `ifelse(condition, FALSE, FALSE)`

The first two cases can be simplified to `condition` and `!condition`
respectively. The last two cases are very likely to be mistakes since the
output is always the same.

This rule has a safe fix and doesn't handle calls to `dplyr::if_else()` and
`data.table::fifelse()` when they have additional arguments.

## Example

```r
ifelse(x %in% letters, TRUE, FALSE)
dplyr::if_else(x > 1, FALSE, TRUE)
```

Use instead:
```r
x %in% letters
!(x > 1) # (or `x <= 1`)
```
