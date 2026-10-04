# yoda_condition

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Checks for literals on the left of `==`, `!=`, `<`, `<=`, `>`, and `>=`,
or supplied as `object` in `expect_equal()`, `expect_identical()`, and
`expect_setequal()`. Also reports comparisons where both values are literals.

## Why is this bad?

Putting the expected value before the actual result, as in `1 == x`, is
called a "Yoda condition". Writing the actual result first makes comparisons
easier to read and testthat failure messages clearer.

Comparisons of two literals, such as `1 == 1` or `expect_equal(1, 1)`, do not
test the result of application code and cannot be fixed automatically.

This rule is **disabled by default**. Select it either with the rule name
`"yoda_condition"` or with the rule group `"READ"`.

Automatic fixes require `--unsafe-fixes`: swapping values can change custom
comparison methods, expectation return values, or tolerance-based results.

## Example

```r
1 == x
10 < length(x)
expect_equal(2, length(x))
```

Use instead:
```r
x == 1
length(x) > 10
expect_equal(length(x), 2)
```
