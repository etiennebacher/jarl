# expect_match
## What it does

Checks for usage of `expect_true(grepl(...))`.

## Why is this bad?

`expect_match()` is more explicit and clearer in intent than wrapping
`grepl()` in `expect_true()`. It also provides better error messages when
tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_match"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_true(grepl("foo", x))
expect_true(base::grepl("bar", x))
```

Use instead:
```r
expect_match(x, "foo")
expect_match(x, "bar")
```
