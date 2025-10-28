# redundant_equals
## What it does

Checks for usage of `==` and `!=` where one of the sides of the operation
is `TRUE` or `FALSE`.

## Why is this bad?

Testing `x == TRUE` is redundant if `x` is a logical vector. Wherever this
is used to improve readability, the solution should instead be to improve
the naming of the object to better indicate that its contents are logical.
This can be done using prefixes (is, has, can, etc.). For example,
`is_child`, `has_parent_supervision`, `can_watch_horror_movie` clarify
their logical nature, while `child`, `parent_supervision`,
`watch_horror_movie` don't.

## Example

```r
x <- c(TRUE, FALSE)
if (any(x == TRUE)) {
  print("hi")
}
```

Use instead:
```r
x <- c(TRUE, FALSE)
if (any(x)) {
  print("hi")
}
```
