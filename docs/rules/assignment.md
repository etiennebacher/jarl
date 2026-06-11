# assignment
::: {.callout-note title="Added in 0.0.8" .low-opacity}
:::

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

If the `operator` parameter is `"="` then replace:
```r
x <- "a"
```
by:
```r
x = "a"
```

Note that Jarl will not report some cases where `<-` is used because it
would change the meaning of code, e.g. this:

```r
f(x <- 1)
```
cannot be replaced by:

```r
f(x = 1)
```

## References

See:

- [https://style.tidyverse.org/syntax.html#assignment-1](https://style.tidyverse.org/syntax.html#assignment-1)
