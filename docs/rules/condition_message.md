# condition_message
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for calls to `stop()` or `warning()` that contain `paste0()`.

## Why is this bad?

By default, `stop()` and `warning()` concatenate elements in the message
without any separator. Using `paste0()` is therefore not needed.

## Example

```r
stop(paste0('hello ', 'there'))
warning(paste0('hello ', 'there'))
```

```r
stop('hello ', 'there')
warning('hello ', 'there')
```
