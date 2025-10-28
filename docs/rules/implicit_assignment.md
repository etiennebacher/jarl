# implicit_assignment
## What it does

Checks for implicit assignment in function calls and other situations.

## Why is this bad?

Assigning inside function calls or other situations such as in `if()` makes
the code difficult to read, and should be avoided.

## Example

```r
mean(x <- c(1, 2, 3))
x

if (any(y <- x > 0)) {
  print(y)
}
```

Use instead:
```r
x <- c(1, 2, 3)
mean(x)
x

larger <- x > 0
if (any(larger)) {
  print(larger)
}
```

## References

See:

- [https://style.tidyverse.org/syntax.html#assignment](https://style.tidyverse.org/syntax.html#assignment)
