# string_boundary
## What it does

Checks for `substr()` and `substring()` calls that can be replaced with
`startsWith()` or `endsWith()`.

## Why is this bad?

Using `startsWith()` and `endsWith()` is both more readable and more efficient
than extracting substrings and comparing them.

This rule has a safe fix.

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
