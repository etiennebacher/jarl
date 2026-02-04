# for_loop_dup_index
## What it does

Checks whether the index symbol in a `for` loop is already used in a parent
`for` loop.

## Why is this bad?

In nested loops, using the same index symbol for several loops can lead to
unexpected and incorrect results.

This rules doesn't have an automatic fix.

## Example

```r
for (x in 1:3) {
  for (x in 1:4) {
    print(x + 1)
  }
}
```

```r
for (x_outer in 1:3) {
  for (x_inner in 1:4) {
    print(x_inner + 1)
  }
}
```
