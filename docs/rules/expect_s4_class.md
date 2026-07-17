# expect_s4_class
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for usage of `expect_true(is(x, "y"))`.

## Why is this bad?

`expect_s4_class()` is designed specifically for testing the class of S4
objects. It makes the intent clearer and provides better error messages when
the test fails.

This rule is **disabled by default**. Select it either with the rule name
`"expect_s4_class"` or with the rule group `"TESTTHAT"`.

This rule has a safe automatic fix but doesn't report calls that pass
`info` or `label` to `expect_true()`.

## Example

```r
expect_true(is(x, "Matrix"))
```

Use instead:
```r
expect_s4_class(x, "Matrix")
```
