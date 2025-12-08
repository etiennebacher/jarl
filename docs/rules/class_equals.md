# class_equals
## What it does

Checks for usage of `class(...) == "some_class"`,
`class(...) %in% "some_class"`, and `identical(class(...), "some_class")`.

For `==` and `%in%` operators, the only cases that are flagged (and potentially
fixed) are cases that:

- happen in the condition part of an `if ()` statement or of a `while ()`
  statement,
- and are not nested in other calls.

For example, `if (class(x) == "foo")` would be reported, but not
`if (my_function(class(x) == "foo"))`.

For `identical()` calls, all cases are flagged regardless of context.

## Why is this bad?

An R object can have several classes. Therefore,
`class(...) == "some_class"` would return a logical vector with as many
values as the object has classes, which is rarely desirable.

It is better to use `inherits(..., "some_class")` instead. `inherits()`
checks whether any of the object's classes match the desired class.

The same rationale applies to `class(...) %in% "some_class"`. Similarly,
`identical(class(...), "some_class")` would break if a class is added or
removed to the object being tested.

## Example

```r
x <- lm(drat ~ mpg, mtcars)
class(x) <- c("my_class", class(x))

if (class(x) == "lm") {
  # <do something>
}

identical(class(x), "foo")
```

Use instead:
```r
x <- lm(drat ~ mpg, mtcars)
class(x) <- c("my_class", class(x))

if (inherits(x, "lm")) {
  # <do something>
}

inherits(x, "foo")
```

## References

See `?inherits`
