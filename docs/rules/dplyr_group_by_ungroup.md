# dplyr_group_by_ungroup
::: {.callout-note title="Added in [0.5.0](https://github.com/etiennebacher/jarl/releases/tag/0.5.0)" .low-opacity}
:::

## What it does

Checks for `group_by() |> verb() |> ungroup()` patterns that can be
simplified using the `.by` or `by` argument.

## Why is this bad?

Since `dplyr` 1.1.0, verbs like `summarize()`, `mutate()`, `filter()`,
`reframe()`, and the `slice_*()` family support a `.by` or `by` argument.
Using `.by` / `by` is shorter and does not require a subsequent `ungroup()`
call.

## Example

```r
x |>
  group_by(grp) |>
  slice_head(mean_val = mean(val)) |>
  ungroup()

x |>
  group_by(grp1, grp2) |>
  summarize(mean_val = mean(val)) |>
  ungroup()
```

Use instead:
```r
x |>
  slice_head(mean_val = mean(val), by = grp)

x |>
  summarize(mean_val = mean(val), .by = c(grp1, grp2))
```

## References

See the `.by` argument in `?dplyr::summarize`.
