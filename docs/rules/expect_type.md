# expect_type
## What it does

Checks for usage of `expect_equal(typeof(x), type)`,
`expect_identical(typeof(x), type)`, and `expect_true(is.<type>(x))` in tests.

## Why is this bad?

`expect_type()` is more explicit and clearer in intent than comparing with
`expect_equal()`, `expect_identical()`, or wrapping type checks in
`expect_true()`. It also provides better error messages when tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_type"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_equal(typeof(x), "double")
expect_identical(typeof(x), "integer")
expect_true(is.character(x))
```

Use instead:
```r
expect_type(x, "double")
expect_type(x, "integer")
expect_type(x, "character")
```
