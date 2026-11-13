# yoda_condition

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Checks for literals on the left of `==`, `!=`, `<`, `<=`, `>`, and `>=`,
or supplied as `object` in `expect_equal()`, `expect_identical()`, and
`expect_setequal()`.

## Why is this bad?

Putting the expected value before the actual result, as in `1 == x`, is
called a "Yoda condition". Writing the actual result first makes comparisons
easier to read.

This rule has an unsafe automatic fix because swapping values can change
results, for instance due to S3 method dispatch.

## Example

```r
1 == x
10 < length(x)
expect_equal(2, x + y)
```

Use instead:
```r
x == 1
length(x) > 10
expect_equal(x + y, 2)
```

## References

* https://en.wikipedia.org/wiki/Yoda_conditions
