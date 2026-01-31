# unexplained_suppression
## What it does

Checks for suppression comments that are missing an explanation.

## Why is this bad?

Suppression comments without explanations make it hard to understand why a
rule was suppressed. Over time, these unexplained suppressions can lead to
technical debt as developers may not know if the suppression is still needed
or what the original reason was.

A `# jarl-ignore` comment without an explanation is ignored by Jarl.

## Example

```r
# The comment below isn't applied, the code below is still reported.
# jarl-ignore any_is_na
any(is.na(x))
```

Use instead:
```r
# jarl-ignore any_is_na: <reason>
any(is.na(x))
```
