# misplaced_suppression
## What it does

Checks for suppression comments placed at the end of a line.

## Why is this bad?

End-of-line suppression comments (trailing comments) are not supported by
Jarl because the comment system attaches them to the expression they follow,
not to the next expression. This means the suppression would not apply to
the intended code.

## Example

```r
# The comment below isn't applied because it's at the end of a line.
any(is.na(x)) # jarl-ignore any_is_na: <reason>
```

Use instead:
```r
# jarl-ignore any_is_na: <reason>
any(is.na(x))
```
