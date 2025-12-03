# sprintf
## What it does

Multiple checks for `sprintf()`:

1. checks whether the `fmt` argument is a constant string (in which case
   `sprintf()` is not needed);
2. checks whether there is a mismatch between the number of special characters
   and the number of arguments;
3. checks whether the `fmt` argument contains invalid special characters.

## Why is this bad?

For 1, using `sprintf()` with a constant string, e.g. `sprintf("abc")`, is
useless and less readable. This has a safe fix that extracts the string.

For 2, a mismatch between the number of special characters and the number of
arguments would generate a runtime error or a warning:

- if the number of special characters > number of arguments, it errors, e.g.
  `sprintf("%s %s", "a")`;
- otherwise, it warns, e.g. `sprintf("%s", "a", "b")`.

For 3, passing invalid special characters would error at runtime, e.g.
`sprintf("%y", "a")`.

Cases 2 and 3 do not have an automatic fix.

## Example

```r
sprintf("abc")
sprintf("%s %s", "a") # would error
sprintf("%y", "a") # would error
```

Use instead:
```r
"abc"
```

## References

See `?sprintf`
