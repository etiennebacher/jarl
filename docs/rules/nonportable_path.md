# nonportable_path
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for likely file paths constructed with hard-coded `/` or `\\`
separators.

## Why is this bad?

Hard-coded path separators may not work consistently across operating
systems. Use `file.path()` to construct portable paths.

This rule is disabled by default because this heuristic can also match
regular expressions and other strings that are not file paths.

This rule uses a conservative heuristic: a separator must be followed by a
path component containing at least two characters. URLs, root paths, and
strings containing characters that are generally invalid in paths are
ignored. Strings inside known regular-expression and date-formatting
functions, or arguments named `pattern` or `format`, are also ignored.

## Example

```r
path <- "data/raw/input.csv"
```

Use instead:
```r
path <- file.path("data", "raw", "input.csv")
```

## References

See `?file.path`.
