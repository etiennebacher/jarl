# yoda_test

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Checks for literals supplied as `object` in `expect_equal()`,
`expect_identical()`, and `expect_setequal()`. Also reports comparisons
where both arguments are literals.

## Why is this bad?

Testthat expectations take the actual result first and the expected value
second. Putting the expected value first is called a "Yoda test". Following
the usual order makes tests easier to read and failure messages clearer.

Comparisons of two literals, such as `expect_equal(1, 1)`, do not test the
result of any application code. These require a meaningful test instead
and cannot be fixed automatically.

This rule is **disabled by default**. Select it either with the rule name
`"yoda_test"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_equal(2, length(x))
expect_identical("a", get_name(x))
expect_setequal(1L, unique(x))
```

Use instead:
```r
expect_equal(length(x), 2)
expect_identical(get_name(x), "a")
expect_setequal(unique(x), 1L)
```
