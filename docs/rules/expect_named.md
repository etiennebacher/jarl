# expect_named
## What it does

Checks for usage of `expect_equal(names(x), n)` and `expect_identical(names(x), n)`.

## Why is this bad?

`expect_named(x, n)` is more explicit and clearer in intent than using
`expect_equal()` or `expect_identical()` with `names()`. It also provides
better error messages when tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_named"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_equal(names(x), "a")
expect_identical(names(x), c("a", "b"))
expect_equal("a", names(x))
```

Use instead:
```r
expect_named(x, "a")
expect_named(x, c("a", "b"))
expect_named(x, "a")
```
