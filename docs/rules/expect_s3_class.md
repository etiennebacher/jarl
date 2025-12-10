# expect_s3_class
## What it does

Checks for usage of `expect_equal(class(x), "y")` and
`expect_identical(class(x), "y")`.

## Why is this bad?

`expect_equal(class(x), "y")` will fail if `x` gets more classes in the future,
even if `"y"` is still one of those classes. `expect_s3_class(x, "y")`
is more robust because the test success doesn't depend on the number or
on the order of classes of `x`. This function also gives clearer error
messages in case of failure.

To test that `x` only has the class `"y"`, then one can use
`expect_s3_class(x, "y", exact = TRUE)`.

This rule is **disabled by default**. Select it either with the rule name
`"expect_s3_class"` or with the rule group `"TESTTHAT"`.

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

Finally, the intent of the test cannot be inferred with the code only, so
the user will have to add `exact = TRUE` if necessary.

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
