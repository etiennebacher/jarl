# if_constant_condition

## What it does

Detects `if` conditions that are always `TRUE` or `FALSE` when there is no
`else` clause.

## Why is this bad?

Code with constant conditions will either never run or always run. It clutters
the code and makes it more difficult to read. Dead code should be removed and
always true code should be unwrapped.

## Example

```r
if (TRUE) {
  print("always true")
}

if (FALSE && ...) {
  print("always false")
}

if (TRUE || ...) {
  print("always true")
}
```

Use instead:

```r
print("always true")

# If always false needed for debugging:
# if (FALSE && ...) {
#   print("always false")
# }
```
