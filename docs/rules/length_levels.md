# length_levels
## What it does

Check for `length(levels(...))` and replace it with `nlevels(...)`.

## Why is this bad?

`length(levels(...))` is harder to read `nlevels(...)`.

Internally, `nlevels()` calls `length(levels(...))` so there are no
performance gains.

## Example

```r
x <- factor(1:3)
length(levels(x))
```

Use instead:
```r
x <- factor(1:3)
nlevels(x)
```
