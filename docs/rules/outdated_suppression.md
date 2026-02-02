# outdated_suppression
## What it does

Checks for suppression comments that don't suppress any actual violations.

## Why is this bad?

Suppression comments that are no longer needed can be confusing and may
indicate that the underlying code has changed but the comment was not
updated. They also add noise to the codebase.

## Example

```r
# The suppression below is unnecessary because there's no any_is_na violation.
# jarl-ignore any_is_na: <reason>
x <- 1
```

Use instead:
```r
# Remove the suppression comment since it's not needed.
x <- 1
```
