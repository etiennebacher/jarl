# undesirable_function
## What it does

Checks for calls to functions listed as undesirable.

## Why is this bad?

Some functions should not appear in production code. For example,
`browser()` is a debugging tool that interrupts execution, and should be
removed before committing.

## Configuration

By default, only `browser` is flagged. You can customise the list in
`jarl.toml`:

```toml
[lint.undesirable-function]
# Replace the default list entirely:
functions = ["browser", "debug"]

# Or add to the defaults:
extend-functions = ["debug"]
```

## Example

```r
do_something <- function(abc = 1) {
   xyz <- abc + 1
   browser()      # flagged by default
   xyz
}
```
