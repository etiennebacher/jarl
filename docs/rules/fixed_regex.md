# fixed_regex
## What it does

Checks for regex functions (`grep`, `grepl`, `gsub`, `sub`, `regexpr`,
`gregexpr`, `regexec`) called with a pattern that contains no special
regex characters and without `fixed = TRUE`.

## Why is this bad?

When a pattern contains no special regex characters, using `fixed = TRUE`
provides a significant performance boost because it uses simple string
matching instead of regex engine pattern matching.

This rule has a safe automatic fix.

## Example

```r
grep("hello", x)
gsub("world", "universe", text)
```

Use instead:
```r
grep("hello", x, fixed = TRUE)
gsub("world", "universe", text, fixed = TRUE)
```

## References

See `?grep` and `?fixed`
