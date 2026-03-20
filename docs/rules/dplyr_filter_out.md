# dplyr_filter_out
## What it does

Checks `dplyr::filter()` calls with complex conditions and suggests using
`dplyr::filter_out()` instead.

## Why is this bad?

Using `filter()` with negated conditions can be hard to read, especially
when we also want to retain missing values. `filter(!condition)` drops rows
where `condition` is `TRUE` **and** rows where it is `NA`, meaning that if
we want to retain those then we have to complement the condition with
`is.na()`:

```r
# We want to drop rows whose value for `col` is larger than the average
# of `col`:
larger_than_average <- function(x) x > mean(x, na.rm = TRUE)
x |> filter(!larger_than_average(col) | is.na(larger_than_average(col)))
```

`dplyr` 1.2.0 introduced `filter_out()` as a complement to `filter()`.
`filter_out()` drops rows that match the condition, meaning that rows where
the condition is `NA` are retained. We can then rewrite the code above like
this:

```r
x |> filter_out(larger_than_average(col))
```

This rule suggests an automatic fix to rewrite them with `filter_out()`. It
is only valid for `dplyr` >= 1.2.0, and only works on `filter()` calls where
all conditions are made of one expression + `is.na()` on the same column.

## Example

```r
library(dplyr)
x <- tibble(a = c(1, 2, 2, NA), b = c(1, 1, 2, 3))

x |> filter(a > 1 | is.na(a))

x |> filter(a > 1 | is.na(a), is.na(b) | b <= 2)
```

Use instead:
```r
library(dplyr)
x <- tibble(a = c(1, 2, 2, NA), b = c(1, 1, 2, 3))

x |> filter_out(a <= 1)

x |> filter_out(a <= 1 | b > 2)
```

## References

- <https://dplyr.tidyverse.org/reference/filter.html>
