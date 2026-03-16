# dplyr_filter_out
## What it does

Checks for negations inside `dplyr::filter()` that can be replaced with
`dplyr::filter_out()`.

## Why is this bad?

`filter(!condition)` drops rows where `condition` is `TRUE` **and** rows
where it is `NA`. `filter_out(condition)` drops only `TRUE` rows, keeping
`NA`s. Using `filter_out()` avoids accidentally dropping `NA` rows and
removes the need for verbose `| is.na()` guards.

## Details

`filter_out()` was introduced in dplyr 1.2.0.

Note that `filter(!cond)` and `filter_out(cond)` handle `NA` values
differently: `filter()` drops `NA` rows while `filter_out()` keeps them.
The automatic fix is only applied for the `cond | is.na(var)` pattern,
where the replacement is semantically equivalent. For plain negations
(`filter(!cond)`), only a diagnostic is emitted.

## Example

```r
library(dplyr)
x <- tibble(a = c(1, 2, 2, NA), b = c(1, 1, 2, 3))

x |> filter(a > 1 | is.na(a))

x |> filter(a > 1 | is.na(a), b < 2)
```

Use instead:
```r
library(dplyr)
x <- tibble(a = c(1, 2, NA))

x |> filter_out(a <= 1)
```

## References

- <https://dplyr.tidyverse.org/reference/filter.html>
