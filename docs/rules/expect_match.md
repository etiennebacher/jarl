# expect_match
## What it does

Checks for usage of `expect_true(grepl(...))`.

## Why is this bad?

`expect_match()` is more explicit and clearer in intent than wrapping
`grepl()` in `expect_true()`. It also provides better error messages when
tests fail.

This rule is **disabled by default**. Select it either with the rule name
`"expect_match"` or with the rule group `"TESTTHAT"`.

This rule has an automatic fix but the fix is disabled if `grepl()`
arguments other than `pattern` and `x` are unnamed.

## Example

```r
expect_true(grepl("foo", x))
expect_true(grepl("bar", x, perl = FALSE, fixed = FALSE))
```

Use instead:
```r
expect_match(x, "foo")
expect_match(x, "bar", perl = FALSE, fixed = FALSE)
```
