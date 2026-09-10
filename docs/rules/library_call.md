# library_call
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Reports `library()` calls that are not grouped at the top of the script,
and moves them there.

A preamble of setup code is allowed before the first `library()` call;
what matters is that every `library()` call in the script forms a single
consecutive block starting at the first one.

Only `library()` is considered: `require()` returns a value that is
routinely used for its result, so it is out of scope for this rule.

This rule is skipped in R Markdown and Quarto documents, where it is often
more acceptable to have `library()` calls in various chunks.

## Why is this bad?

Scripts where `library()` calls are scattered between the code are hard to
read: a reader cannot tell at a glance which packages the script needs,
and the attach order (which decides masking) becomes accidental rather
than deliberate.

This rule has an unsafe fix: moving `library()` calls changes attach order
and therefore which package masks which.

This rule is disabled by default.

## Example

```r
library(dplyr)
x <- 1
library(purrr)
```

Use instead:
```r
library(dplyr)
library(purrr)
x <- 1
```
