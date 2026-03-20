# browser
## What it does

Checks for lingering presence of `browser()` which should not be present in
released code.

**This rule is deprecated and will be removed in a future version. Use the
rule [`undesirable_function`](https://jarl.etiennebacher.com/rules/undesirable_function)
and configure it to report calls to `browser()` instead.**

## Why is this bad?

`browser()` interrupts the execution of an expression and allows the inspection
of the environment where `browser()` was called from. This is helpful while
developing a function, but is not expected to be called by the user. Does not
remove the call as it does not have a suitable replacement.

## Example

```r
do_something <- function(abc = 1) {
   xyz <- abc + 1
   browser()      # This should be removed.
   xyz
}

```

## References

See `?browser`
