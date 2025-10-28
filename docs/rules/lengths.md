# lengths
## What it does

Checks for usage of `length()` in several functions that apply it to each
element of a list, such as `lapply()`, `vapply()`, `purrr::map()`, etc.,
and replaces it with `lengths()`.

## Why is this bad?

`lengths()` is faster and more memory-efficient than applying `length()`
on each element of the list.

## Example

```r
x <- list(a = 1, b = 2:3, c = 1:10)
sapply(x, length)
```

Use instead:
```r
x <- list(a = 1, b = 2:3, c = 1:10)
lengths(x)
```

## References

See `?lengths`
