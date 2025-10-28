# length_test
## What it does

Checks for usage of `length(... == some_val)` and replaces it with
`length(...) == some_val`.

## Why is this bad?

This is very likely a mistake since computing the length of the output of
`==` is the same as computing the length of the inputs.

## Example

```r
x <- 1:3
length(x == 1)
```

Use instead:
```r
x <- 1:3
length(x) == 1
```
