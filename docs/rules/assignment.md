# assignment
## What it does

Checks for consistency of assignment operator.

## Why is this bad?

In most cases using `=` and `<-` is equivalent. Some very popular packages
use `=` without problems. This rule only ensures the consistency of the
assignment operator in a project.

Note that Jarl doesn't force you to use `<-` as assignment operator, it
simply uses it as default. To use `=` as the preferred operator:

- in the CLI (temporary change), use `--assignment "="`;
- in `jarl.toml` (permanent change): set `assignment = "="`.

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
