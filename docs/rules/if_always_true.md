# if_always_true
## What it does

Detects `if` conditions that always evaluate to `TRUE`. This is only triggered
for `if` statements without an `else` clause, these are handled by
`unreachable_code`.

## Why is this bad?

Code in an `if` statement whose condition always evaluates to `TRUE` will
always run. It clutters the code and makes it more difficult to read. In
these cases, the `if` condition should be removed.

This rule does not have an automatic fix.

## Example

```r
if (TRUE) {
  print("always true")
}

if (TRUE || ...) {
  print("always true")
}

if (!FALSE) {
  print("always true")
}
```

Use instead:

```r
print("always true")
```
