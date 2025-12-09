# expect_s3_class
## What it does

Checks for usage of `expect_equal(class(x), "y")` and
`expect_identical(class(x), "y")`.

## Why is this bad?

`expect_equal(class(x), "y")` will fail if `x` gets more classes in the future,
even if `"y"` is still one of those classes. It is more robust to use
`expect_s3_class(x, "y")` instead since it doesn't depend on the number or
order of classes of `x`. It also gives clearer error messages in case of
failure.

This rule is **disabled by default**. Select it either with the rule name
`"expect_named"` or with the rule group `"TESTTHAT"`.

This rule has a safe automatic fix but doesn't report cases where:

* `expect_s3_class()` would fail, such as:
  ```r
  testthat::expect_s3_class(list(1), "list")
  testthat::expect_s3_class(1L, "integer")
  ```
  For those cases, it is recommended to use `expect_type()` instead.

* the `expected` object could have multiple values, such as:
  ```r
  testthat::expect_equal(class(x), c("foo", "bar"))
  testthat::expect_equal(class(x), vec_of_classes)
  ```

## Example

```r
expect_equal(class(x), "data.frame")
expect_identical(class(x), "Date")
```

Use instead:
```r
expect_s3_class(x, "data.frame")
expect_s3_class(x, "Date")
```
