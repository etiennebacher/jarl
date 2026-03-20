# assignment
## What it does

Checks for consistency of assignment operator.

## Why is this bad?

In most cases using `=` and `<-` is equivalent. Some very popular packages
use `=` without problems. This rule only ensures the consistency of the
assignment operator in a project.

Set the following option in `jarl.toml` to use `=` as the preferred operator:

```toml
[lint.assignment]
operator = "=" # or "<-"
```

## Example

```r
x = "a"
```

Use instead:
```r
x <- "a"
```

## References

See:

- [https://style.tidyverse.org/syntax.html#assignment-1](https://style.tidyverse.org/syntax.html#assignment-1)
