# unmatched_range_suppression
## What it does

Checks for `jarl-ignore-start` and `jarl-ignore-end` comments that don't have
a matching counterpart at the same nesting level.

## Why is this bad?

Start and end suppression comments must be matched at the same nesting level.
A start comment inside a function body cannot be closed by an end comment
outside that function, and vice versa. Unmatched suppressions indicate a
mistake in the suppression structure and may not suppress what you intended.

## Example

```r
# The start and end are at different nesting levels, so both are unmatched.
# jarl-ignore-start any_is_na: <reason>
f <- function() {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}
any(is.na(x))  # This is NOT suppressed!
```

Use instead:
```r
# Start and end at the same level
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na
```
