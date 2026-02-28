# expect_no_match
## What it does

Checks for usage of `expect_false(grepl(...))`.

## Why is this bad?

`expect_no_match()` is more explicit and clearer in intent than wrapping
`grepl()` in `expect_false()`. It also provides better error messages when
tests fail.

Note: negated forms like `expect_false(!grepl(...))` are intentionally
ignored by this rule and handled by `expect_not`.

This rule is **disabled by default**. Select it either with the rule name
`"expect_no_match"` or with the rule group `"TESTTHAT"`.

## Example

```r
expect_false(grepl("foo", x))
expect_false(grepl("bar", x, perl = FALSE, fixed = FALSE))
```

Use instead:
```r
expect_no_match(x, "foo")
expect_no_match(x, "bar", perl = FALSE, fixed = FALSE)
```
