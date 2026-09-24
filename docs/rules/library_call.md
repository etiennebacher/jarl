# library_call

::: {.callout-note title="Added in 0.7.0" .low-opacity}
:::

## What it does

Reports `library()` calls that are not grouped at the top of the script.

## Why is this bad?

Scripts where `library()` calls are scattered between the code are hard to
read: a reader cannot tell at a glance which packages the script needs. This
rule has several exceptions:

- an `if` statement that only contains `library()` calls is considered
  equivalent to a simple `library()` call;
- `options()` and `Sys.setenv()` at the top of the script stay there. Jarl
  will not move `library()` calls above these functions.
- this rule is skipped in R Markdown and Quarto documents, where it is more
  common to have `library()` calls in various chunks.

This rule has an unsafe fix that moves `library()` calls towards the top.
This is unsafe because code that would initially run before some `library()`
calls would run after and therefore could be affected by new namespace
conflicts.

This rule is disabled by default.

## Example

```r
library(dplyr)
x <- 1
library(purrr)
y <- 2
library(data.table)
```

Use instead:
```r
library(dplyr)
library(purrr)
library(data.table)
x <- 1
y <- 2
```
