# loop_invariant

::: {.callout-note title="Added in 0.8.0" .low-opacity}
:::

## What it does

Reports assignments in the body of a `for` loop whose value is the same at
every iteration, because it doesn't depend on the loop variable or on any
object modified in the loop.

This rule is conservative: it only reports top-level assignments whose
value is built from a known set of base R functions that have no side
effects, and it skips loops that contain calls able to modify objects in
ways the analysis can't follow (such as `assign()`, `eval()`, or `<<-`).

## Why is this bad?

The value is recomputed at every iteration even though it never changes.
Computing it once before the loop is faster and makes clearer what actually
depends on the loop.

Note that moving the assignment before the loop also runs it when the loop
has zero iterations.

## Example

```r
for (i in seq_along(files)) {
  n <- nrow(data)
  out[[i]] <- process(files[i], n)
}
```

Use instead:

```r
n <- nrow(data)
for (i in seq_along(files)) {
  out[[i]] <- process(files[i], n)
}
```
