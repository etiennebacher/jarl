# class_equals
## What it does

Checks for usage of `class(...) == "some_class"` and
`class(...) %in% "some_class"`. The only cases that are flagged (and
potentially fixed) are cases that:

- happen in the condition part of an `if ()` statement or of a `while ()`
  statement,
- and are not nested in other calls.

For example, `if (class(x) == "foo")` would be reported, but not
`if (my_function(class(x) == "foo"))`.

## Why is this bad?

An R object can have several classes. Therefore,
`class(...) == "some_class"` would return a logical vector with as many
values as the object has classes, which is rarely desirable.

It is better to use `inherits(..., "some_class")` instead. `inherits()`
checks whether any of the object's classes match the desired class.

The same rationale applies to `class(...) %in% "some_class"`.

## Example

```r
x <- lm(drat ~ mpg, mtcars)
class(x) <- c("my_class", class(x))

if (class(x) == "lm") {
  # <do something>
}
```

Use instead:
```r
x <- lm(drat ~ mpg, mtcars)
class(x) <- c("my_class", class(x))

if (inherits(x, "lm")) {
  # <do something>
}
```

## References

See `?inherits`
