# outer_negation
## What it does

Checks for usage of `all(!x)` or `any(!x)`.

## Why is this bad?

Those two patterns may be hard to read and understand, especially when the
expression after `!` is lengthy. Using `!any(x)` instead of `all(!x)` and
`!all(x)` instead of `any(!x)` may be more readable.

In addition, using the `!` operator outside the function call is more
efficient since it only has to invert one value instead of all values inside
the function call.

## Example

```r
any(!x)
all(!x)
```

Use instead:
```r
!all(x)
!any(x)
```
