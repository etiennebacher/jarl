# condition_call
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Checks for calls to `stop()` that display the call in the error message,
either because `call.` is not set (it defaults to `TRUE`) or because it is
explicitly set to `TRUE`.

## Why is this bad?

By default, `stop()` shows the call that triggered the error in the
message. This can be noisy and lead to confusion if the user didn't directly
call the function that threw the error.

## Example

```r
internal_function <- function(x) {
  if (x < 5) {
    stop("x lower than 5")
  }
}

external_function<- function(x) {
  out <- internal_function(x)
  # do something with `out`...
}

external_function(1)
#> Error in `internal_function()`:
#> x lower than 5
```

In this case, the error message is slightly confusing because the user never
called `internal_function()` directly. With `call. = FALSE` instead:

```r
internal_function <- function(x) {
  if (x < 5) {
    stop("x lower than 5", call. = FALSE)
  }
}

external_function<- function(x) {
  out <- internal_function(x)
  # do something with `out`...
}

external_function(1)
#> Error:
#> x lower than 5
```

## References

* https://design.tidyverse.org/err-call.html
