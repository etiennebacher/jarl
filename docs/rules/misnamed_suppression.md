# misnamed_suppression
## What it does

Checks for suppression comments with an invalid rule name.

## Why is this bad?

A suppression comment with an unrecognized rule name will not suppress any
violations. This could be due to a typo in the rule name or using a rule
name that doesn't exist.

## Example

```r
# The comment below isn't applied because "any_isna" is not a valid rule.
# jarl-ignore any_isna: <explanation>
any(is.na(x))
```

Use instead:
```r
# jarl-ignore any_is_na: <explanation>
any(is.na(x))
```
