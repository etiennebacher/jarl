# nzchar
## What it does

Checks for usage of `x != ""` or `x == ""`
 instead of `nzchar(x)` or `!nzchar(x)`.

## Why is this bad?

One crucial difference is in the default handling of `NA_character_`,
i.e., missing strings. `nzchar(NA_character_)` is TRUE,
while `NA_character_ == ""` and `nchar(NA_character_) == 0` are both NA.
Therefore, for strict compatibility, use `nzchar(x, keepNA = TRUE)`.
If the input is known to be complete (no missing entries),
this argument can be dropped for conciseness.

This rule comes with a unsafe fix.

## Example

```r
if (x == "") {
  message("empty string")
}
```

Use instead:
```r
if (!nzchar(x)) {
  message("empty string")
}
```

## References

See `?nzchar`
