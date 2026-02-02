# if_always_true

## What it does

Detects `if` conditions that are always `TRUE`when there is no `else` clause.

## Why is this bad?

Code with constant TRUE conditions will always run. It clutters the code and
makes it more difficult to read. Always true code should be unwrapped.

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
