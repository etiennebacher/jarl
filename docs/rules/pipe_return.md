# pipe_return
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Reports `return()` used on the right-hand side of the `magrittr` pipe
`%>%`, whether written with parentheses (`x %>% return()`) or as a bare
identifier (`x %>% return`).

The native pipe `|>` is not considered because `x |> return()` is a syntax
error and would be caught by the parser anyway.

## Why is this bad?

`return()` on the right-hand side of `%>%` does not behave like a regular
`return()` call and doesn't exit the function early. See the examples below.

## Example

```r
f <- function(x) {
  x %>% sum() %>% return()
  1 + 1
}

f(1:3)
#> 2
```

In the example above, the output isn't the sum of `x` but `1 + 1`, even
though we'd expect the `return()` to return the output of `x %>% sum()`.

Wrapping the pipe in `return()` instead is unambiguous:

```r
f <- function(x) {
  return(x %>% sum())
  1 + 1
}

# OR:
f <- function(x) {
  out <- x %>% sum()
  return(out)
  1 + 1
}

f(1:3)
#> 6
```
