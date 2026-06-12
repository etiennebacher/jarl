# nested_pipe
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Reports pipes (`%>%` or `|>`) that are nested inside another function call,
e.g. `print(x %>% foo())`.

## Why is this bad?

Nesting a pipe inside another call hides the data flow and makes the code
harder to read. Extracting the pipe into its own statement keeps each step
on its own line.

`try()`, `tryCatch()`, and `withCallingHandlers()` are automatically skipped.
The list of skipped functions can be customized with [rule-specific arguments](https://jarl.etiennebacher.com/reference/config-file#rule-specific-arguments)
in `jarl.toml`.

## Example

```r
print(x %>% foo() %>% bar())
```

Use instead:
```r
out <- x %>%
  foo() %>%
  bar()

print(out)
```
