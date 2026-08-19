# unused_object
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Detects objects that are defined (i.e. assigned a value) but never used.

## Why is this bad?

Unused assignments are usually a sign of dead code or a bug. Removing them
reduces noise.

## Features

Apart from the standard usage of objects in R (e.g. `x <- 1; print(x)`),
this rule handles the following cases:

- String interpolation in the `glue`, `cli`, and `stringr` packages, e.g.
  in this case `x` is not reported as unused:

  ```r
  x <- 1
  glue::glue("{print(x)}")
  ```

  Custom functions or functions from other packages providing string
  interpolation are not supported.

- The `%<>%` operator from `magrittr` is supported.

- Explicit cross-file analysis: calls to `source()` or `targets::tar_source()`
  are detected, e.g. this doesn't report `x` as unused:

    - `foo.R`:
      ```r
      x <- 1
      source("bar.R")
      ```

    - `bar.R`:
      ```r
      print(x)
      ```

  Similarly, the definition could be made in the sourced file (`bar.R`) and
  the use could be made in the other file (`foo.R`).

- Implicit cross-file analysis. All files in an `R` folder (whether this
  corresponds to an R package or to another project type) are collated and
  share the same namespace, meaning that an object defined in `R/a.R` could
  seamlessly be detected as used in `R/b.R`.

- Some functions that can call other quoted functions (e.g. `do.call()`) are
  supported.

## Limitations

Some cases are deliberately left aside or might be tackled in the future:

- Some functions such as `get()` or `mget()` are not handled.

- Quoted code that is evaluated later may lead to false positives, e.g. this
  would wrongly report `x` as unused:

  ```r
  x <- 1
  e <- quote(x + 1)
  eval(e)
  ```

- `source()` and alike only accept literal paths, not R objects, e.g. this
  isn't handled by Jarl:

  ```r
  for (i in my_paths) source(i)
  ```

## In R Markdown and Quarto files

Jarl bundles all chunks together before running the analysis, meaning that
`unused_object` would properly detect whether an object created in a chunk
is used in another.

There are two other cases to handle:

- objects that are present in a chunk with `eval = FALSE` or `#| eval: false`
  are not marked as "used". For instance, in the following example, the
  object `x` would be reported as unused:

  ````markdown
  ```{{r}}
  x <- 1
  ```

  ```{{r eval = FALSE}}
  print(x)
  ```
  ````

  Note that if the option value is only available at runtime (e.g.
  `eval = my_r_object`) then Jarl assumes that the chunk is evaluated.

- inline R code is taken into account, `` `r x` `` in the text would keep
  `x` from being reported as unused.

## Examples

```r
x <- 1   # unused
print(y)
```
