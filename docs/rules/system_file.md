# system_file
::: {.callout-note title="Added in [0.1.2](https://github.com/etiennebacher/jarl/releases/tag/0.1.2)"}
:::

## What it does

Checks for usage of `system.file(file.path(...))` and replaces it with
`system.file(...)`.

## Why is this bad?

In `system.file()`, all unnamed arguments are already passed to `file.path()`
under the hood, so `system.file(file.path(...))` is redundant and harder to
read.

## Example

```r
system.file(file.path("my_dir", "my_sub_dir"), package = "foo")
```

Use instead:
```r
system.file("my_dir", "my_sub_dir", package = "foo")
```

## References

See `?system.file`
