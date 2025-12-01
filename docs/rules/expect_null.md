# expect_null
## What it does

Checks for usage of `expect_equal(x, NULL)`, `expect_identical(x, NULL)`,
and `expect_true(is.null(x))`.

## Why is this bad

`expect_null()` is more explicit and clearer in intent than comparing with
`expect_equal()`, `expect_identical()`, or wrapping `is.null()` in
`expect_true()`. It also provides better error messages when tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_null"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_equal(x, NULL)
expect_identical(x, NULL)
expect_true(is.null(foo(x)))
```

Use instead:
```r
expect_null(x)
expect_null(x)
expect_null(foo(x))
```
