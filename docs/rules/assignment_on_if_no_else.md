# assignment_on_if_no_else
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Flags assignments whose value is an `if` expression without a final `else`
branch, including an `else if` chain whose final `if` has no `else`, when the
assignment overwrites an earlier value.

## Why is this bad?

When no branch is taken, an `if` expression without a final `else` evaluates
to `NULL`. Assigning that result can unexpectedly overwrite an existing value.

## Example

```r
df <- data.frame()
df <- if (condition) {
  data.frame()
}

value <- 1
value <- if (a) {
  1
} else if (b) {
  2
}
```

Use instead:

```r
if (condition) {
  df <- data.frame()
}

if (a) {
  value <- 1
} else if (b) {
  value <- 2
}
```

If assigning a fallback value is appropriate, add a final `else` branch.

