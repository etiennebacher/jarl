# expect_s3_class
::: {.callout-note title="Added in 0.3.0" .low-opacity}
:::

## What it does

Checks for usage of `expect_equal(class(x), "y")`,
`expect_identical(class(x), "y")`, selected
`expect_true(is.<class>(x))`, and `expect_true(inherits(x, "y"))` calls.

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

This rule has a safe automatic fix for statically supported class names.
Dynamic class expressions are reported without an automatic fix because they
could contain classes that are not supported by `expect_s3_class()`.

This rule doesn't report cases where:

* the `is.*()` predicate does not test an S3 class. For example, `is.matrix(x)` does
  not imply that `x` is an S3 object.

* `expect_s3_class()` would fail, such as:
  ```r
  testthat::expect_s3_class(list(1), "list")
  testthat::expect_s3_class(1L, "integer")
  ```
  For those cases, it is recommended to use `expect_type()` instead.

Finally, the intent of the test cannot be inferred with the code only, so
the user will have to add `exact = TRUE` if necessary.

## Example

```r
expect_equal(class(x), "data.frame")
expect_identical(class(x), "Date")
expect_true(is.factor(x))
expect_true(inherits(x, "foo"))
```

Use instead:
```r
expect_s3_class(x, "data.frame")
expect_s3_class(x, "Date")
expect_s3_class(x, "factor")
expect_s3_class(x, "foo")
```
