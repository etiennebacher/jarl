# nzchar
## What it does

Checks for usage of `x != ""` or `x == ""`
 instead of `nzchar(x)` or `!nzchar(x)`.

## Why is this bad?
`x == ""` is less efficient than `!nzchar(x)`
when x is a large vector of long strings. 

One crucial difference is in the default handling of `NA_character_`,
i.e., missing strings. `nzchar(NA_character_)` is TRUE,
while `NA_character_ == ""` is NA.
Therefore, for strict compatibility, use `nzchar(x, keepNA = TRUE)`.
If the input is known to be complete (no missing entries),
this argument can be dropped for conciseness.

This rule comes with a unsafe fix.

## Example

```r
x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
x[x == ""]
```

Use instead:
```r
x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
x[!nzchar(x)]
```

## References

See `?nzchar`
