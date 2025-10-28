# for_loop_index
## What it does

Checks whether the index symbol in a for loop is already used anywhere in
the sequence of the same for loop.

## Why is this bad?

`for (x in x)` or `for (x in foo(x))` are confusing to read and can lead
to errors.

## Example

```r
x <- c(1, 2, 3)
for (x in x) {
  x + 1
}
```

Use instead:
```r
x <- c(1, 2, 3)
for (xi in x) {
  xi + 1
}
```
