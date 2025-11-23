# seq
## What it does

Checks for `1:length(...)`, `1:nrow(...)`, `1:ncol(...)`, `1:NROW(...)` and
`1:NCOL(...)` expressions. See also [seq2](https://jarl.etiennebacher.com/rules/seq2).

## Why is this bad?

Those patterns are often used to generate sequences from 1 to a given
number. However, when the right-hand side of `:` is 0, then this creates
a sequence `1,0` which is often overlooked.

This rule comes with safe automatic fixes using `seq_along()` or `seq_len()`.

## Example

```r
for (i in 1:nrow(data)) {
  print("hi")
}

for (i in 1:length(data)) {
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
