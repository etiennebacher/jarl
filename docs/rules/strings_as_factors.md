# strings_as_factors
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for calls to `data.frame()` that contain a statically identifiable
character column but do not explicitly set `stringsAsFactors`. This rule
only applies when the project's minimum supported R version is known and is
below R 4.0.0.

## Why is this bad?

Before R 4.0.0, `data.frame()` converted strings to factors by default. From
R 4.0.0 onward, strings remain character vectors by default. Code supporting
versions on both sides of this change can therefore return columns with
different types depending on the R version used.

This rule does not provide an automatic fix because either `TRUE` or `FALSE`
can be the intended value of `stringsAsFactors`.

## Example

```r
data.frame(x = "a")
```

Use one of the following instead:
```r
data.frame(x = "a", stringsAsFactors = TRUE)
data.frame(x = "a", stringsAsFactors = FALSE)
```

## References

See `?data.frame`
and the [R Core discussion](https://developer.r-project.org/Blog/public/2020/02/16/stringsasfactors/).
