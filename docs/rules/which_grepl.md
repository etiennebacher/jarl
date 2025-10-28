# which_grepl
## What it does

Checks for usage of `which(grepl(...))` and replaces it with `grep(...)`.

## Why is this bad?

`which(grepl(...))` is harder to read and is less efficient than `grep()`
since it requires two passes on the vector.

## Example

```r
x <- c("hello", "there")
which(grepl("hell", x))
which(grepl("foo", x))
```

Use instead:
```r
x <- c("hello", "there")
grep("hell", x)
grep("foo", x)
```

## References

See `?grep`
