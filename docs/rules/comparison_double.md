# comparison_double
## What it does

Checks for comparisons to a double value (aka float).

## Why is this bad?

In some cases, floating point inacurracies can lead to unexpected results
when comparing two values that should be equal but are not, e.g.:
```r
x <- 0.1 * 3
x == 0.3
#> [1] FALSE
```

This rule has a safe fix that consists in using `all.equal()` when comparing
to doubles:
```r
isTRUE(all.equal(x, 0.3))
#> [1] TRUE
```

Note that `all.equal()` returns a character value if the equality does not
hold, which is why it is necessary to wrap it in `isTRUE()` to recover the
behavior of `==`.

## Example

```r
x == 1
f(x) == 1.3
```

Use instead:
```r
isTRUE(all.equal(x, 1))
isTRUE(all.equal(f(x), 1.3))
```

# References

See:

- [R FAQ 7.31](https://cran.r-project.org/doc/FAQ/R-FAQ.html#Why-doesn_0027t-R-think-these-numbers-are-equal_003f)
- [https://stackoverflow.com/questions/9508518/why-are-these-numbers-not-equal/9508558](https://stackoverflow.com/questions/9508518/why-are-these-numbers-not-equal/9508558) (contains other links too)
