# expect_not
## What it does

Checks for usage of `expect_true(!x)` and `expect_false(!x)` in tests.

## Why is this bad

Using `expect_false(x)` is clearer and more direct than `expect_true(!x)`,
and vice versa.

This rule is **disabled by default**. Select it either with the rule name
`"expect_not"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_true(!x)
expect_false(!foo(x))
expect_true(!(x && y))

# rlang "!!!" operator is left unmodified
expect_true(!!!x)
```

Use instead:
```r
expect_false(x)
expect_true(foo(x))
expect_false(x && y)

# rlang "!!!" operator is left unmodified
expect_true(!!!x)
```
