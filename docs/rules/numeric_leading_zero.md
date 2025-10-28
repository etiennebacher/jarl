# numeric_leading_zero
## What it does

Checks for double or complex values with a decimal component and a
leading `.`.

## Why is this bad?

While `.1` and `0.1` mean the same thing, the latter is easier to read due
to the small size of the `.` glyph.

## Example

```r
x <- .1
```

Use instead:
```r
x <- 0.1
```
