# string_boundary
::: {.callout-note title="Added in 0.3.0" .low-opacity}
:::

## What it does

Checks for `substr()` and `substring()` calls that can be replaced with
`startsWith()` or `endsWith()`.
Only comparisons to non-empty string literals with matching substring
boundaries are reported. Ordinary strings containing escapes are skipped.

## Why is this bad?

Using `startsWith()` and `endsWith()` is both more readable and more efficient
than extracting substrings and comparing them.

This rule has an unsafe fix because the replacement can drop names and other
attributes, no longer coerces non-character inputs, and may evaluate repeated
expressions fewer times.

## Example

```r
substr(x, 1L, 3L) == "abc"
substring(x, nchar(x) - 2L, nchar(x)) == "xyz"
```
Use instead:
```r
startsWith(x, "abc")
endsWith(x, "xyz")
```

## References

See `?startsWith` and `?substr`
