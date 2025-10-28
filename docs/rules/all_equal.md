# all_equal
## What it does

Checks for dangerous usage of `all.equal(...)`, for instance in `if()`
conditions or `while()` loops.

## Why is this bad?

`all.equal()` returns `TRUE` in the absence of differences but returns a
character string (not `FALSE`) in the presence of differences. Usage of
`all.equal()` without wrapping it in `isTRUE()` are thus likely to generate
unexpected errors if the compared objects have differences. An alternative
is to use `identical()` to compare vector of strings or when exact equality
is expected.

This rule has automated fixes that are marked unsafe and therefore require
passing `--unsafe-fixes`. This is because automatically fixing those cases
can change the runtime behavior if some code relied on the behaviour of
`all.equal()` (likely by mistake).

## Example

```r
a <- 1
b <- 1

if (all.equal(a, b, tolerance = 1e-3)) message('equal')
if (all.equal(a, b)) message('equal')
!all.equal(a, b)
isFALSE(all.equal(a, b))

```

Use instead:
```r
a <- 1
b <- 1

if (isTRUE(all.equal(a, b, tolerance = 1e-3))) message('equal')
if (isTRUE(all.equal(a, b))) message('equal')
!isTRUE(all.equal(a, b))
!isTRUE(all.equal(a, b))
```

## References

See `?all.equal`
