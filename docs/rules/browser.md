# browser
## What it does

Checks for lingering presence of `browser()` which should not be present in 
released code. Does not remove the call as it does not have a suitable 
replacement. One option would be `NULL` but this is possibly also bad.

## Why is this bad?

`browser()` interrupts the execution of an expression and allows the inspection 
of the environment where `browser()` was called from. This is helpful while 
developing a function, but is not expected to be called by the user.

## Example

```r
do_something <- function(abc = 1) {
   xyz <- abc + 1
   browser() # this should not appear in a final release
   xyz
}
```

## References

See:

- [https://rdrr.io/r/base/browser.html](https://rdrr.io/r/base/browser.html)
