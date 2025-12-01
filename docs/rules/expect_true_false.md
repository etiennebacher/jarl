# expect_true_false
## What it does

Checks for usage of `expect_equal(x, TRUE)`, `expect_equal(x, FALSE)`,
`expect_identical(x, TRUE)`, and `expect_identical(x, FALSE)` in tests.

## Why is this bad?

`expect_true()` and `expect_false()` are more explicit and clearer in intent
than comparing with `expect_equal()` or `expect_identical()`. They also
provide better error messages when tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_true_false"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_equal(is.numeric(x), TRUE)
expect_identical(is.character(y), FALSE)
```

Use instead:
```r
expect_true(is.numeric(x))
expect_false(is.character(y))
```
