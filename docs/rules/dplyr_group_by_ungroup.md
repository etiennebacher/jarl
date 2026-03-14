# dplyr_group_by_ungroup
## What it does

Checks for `group_by() |> verb() |> ungroup()` patterns that can be
simplified using the `.by` argument.

## Why is this bad?

Since `dplyr` 1.1.0, verbs like `summarize()`, `mutate()`, `filter()`,
`reframe()`, and the `slice_*()` family support a `.by` argument. Using
`.by` is shorter and does not require a subsequent `ungroup()` call.

## Example

```r
x |>
  group_by(grp) |>
  summarize(mean_val = mean(val)) |>
  ungroup()
```

Use instead:
```r
x |>
  summarize(mean_val = mean(val), .by = grp)
```

## References

See the `.by` argument in `?dplyr::summarize`.
