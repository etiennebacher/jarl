---
title: R Markdown and Quarto
---

As of 0.5.0, Jarl can check R code chunks in R Markdown and Quarto documents.
This comes with a few limitations:

* automatic fixes are not available;
* inline R code isn't analyzed, only code chunks;
* features from the editor integration, such as highlighting diagnostics, are only available when the file is open in source mode, not in visual mode.

All R code chunks of a document are analyzed together, in document order, since this is how they are evaluated when the document is rendered.
Rules that need to know how objects flow through the document, such as `unused_object`, therefore work across chunks: an object created in one chunk and used in a later one is not reported.
Inline R code isn't linted, but the objects it reads are taken into account, so `` `r x` `` in the prose keeps `x` from being reported as unused.

Chunks that don't run when the document is rendered, i.e. those marked `eval = FALSE` or `#| eval: false`, are left out of this: they neither create objects nor use them.
An object only used in such a chunk is therefore still reported as unused.
Their code is checked by all the other rules, as usual.
An `eval` option whose value is an expression, such as `eval = run_it`, is only known at render time, so the chunk is assumed to run.

Chunk options can name objects, either in the chunk header (`` ```{r, fig.cap = my_caption} ``) or with Quarto's `!expr` (`#| eval: !expr run_it`).
Those objects count as used, in every chunk: knitr evaluates a chunk's options even when the chunk itself doesn't run.
Only the values are read, so an object that happens to be named like a chunk option is still reported.

Suppression comments such as `# jarl-ignore` are supported in R code chunks.
In Quarto and R Markdown files, you can also use the comment `#| jarl-ignore-chunk` to ignore specific rules on entire chunks.
Moreover, the comment `# jarl-ignore-file` must be located in the first R code chunk, before any R code.
See [Suppression comments](suppression-comments.md) for more details.

By default, Jarl checks R code chunks in R Markdown and Quarto documents.
To select or ignore particular file extensions, you can use glob patterns in the command line or in `jarl.toml`:

* in the command line:

  ```
  # Analyze R files only, not R Markdown and Quarto files
  jarl check **/*.R

  # Analyze R Markdown and Quarto files only, not R files
  jarl check **/*.{Rmd,rmd,qmd}
  ```

* in `jarl.toml`:

  ```
  [lint]
  ...

  # Analyze R files only, not R Markdown and Quarto files
  include = ["**/*.R"]

  # Analyze R Markdown and Quarto files only, not R files
  include = ["**/*.{Rmd,rmd,qmd}"]
  ```
