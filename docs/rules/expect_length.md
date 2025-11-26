# expect_length
## What it does

Checks for usage of `expect_equal(length(x), n)` and `expect_identical(length(x), n)`.

## Why is this bad?

`expect_length(x, n)` is more explicit and clearer in intent than using
`expect_equal()` or `expect_identical()` with `length()`. It also provides
better error messages when tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_length"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_equal(length(x), 2)
expect_identical(length(x), n)
expect_equal(2L, length(x))
```

Use instead:
```r
expect_length(x, 2)
expect_length(x, n)
expect_length(x, 2L)
```
