# pipe_consistency
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Reports cases where both pipes (`%>%` or `|>`) are used. By default, the
base pipe `|>` is preferred but this can be changed in the configuration
file.

## Why is this bad?

This simply ensures that pipe usage is consistent. There are a few cases
where both pipes are not equivalent, and therefore where this rule doesn't
report diagnostics:

- if the RHS of a `%>%` uses `.` as an unnamed argument, then it is not
  reported because there is no equivalent in base R (the `_` placeholder
  only works for named arguments).

- if the RHS of a `%>%` uses `.` several times, then it is not reported
  because there is no equivalent in base R (the `_` placeholder can only be
  used once in the RHS).

This rule is available only for R >= 4.2 (the `_` placeholder was
introduced in 4.2, even though `|>` itself was introduced in 4.1), and it
has an unsafe fix due to some specificities of the native pipe (e.g. it
doesn't work when `+()` is on the RHS).

## Example

```r
data %>%
  transform(a = x / 2) |>
  plot()
```

Use instead:
```r
data |>
  transform(a = x / 2) |>
  plot()
```

## References

See `?pipeOp`
