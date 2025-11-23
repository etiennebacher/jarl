# seq2
## What it does

Checks for `seq(length(...))`, `seq(nrow(...))`, `seq(ncol(...))`,
`seq(NROW(...))`, `seq(NCOL(...))`. See also [seq](https://jarl.etiennebacher.com/rules/seq).

## Why is this bad?

Those patterns are often used to generate sequences from 1 to a given
number. However, when `length(...)` is 0, then this creates a sequence `1,0`
which is often overlooked.

This rule comes with safe automatic fixes using `seq_along()` or `seq_len()`.

## Example

```r
for (i in seq(nrow(data))) {
  print("hi")
}

for (i in seq(length(data))) {
  print("hi")
}
```

Use instead:
```r
for (i in seq_len(nrow(data))) {
  print("hi")
}

for (i in seq_along(data)) {
  print("hi")
}
```
