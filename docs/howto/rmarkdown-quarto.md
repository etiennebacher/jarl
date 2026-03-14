---
title: R Markdown and Quarto
---

As of 0.5.0, Jarl can check R code chunks in R Markdown and Quarto documents.
This comes with a few limitations:

* automatic fixes are not available;
* inline R code isn't analyzed, only code chunks;
* features from the editor integration, such as highlighting diagnostics, are only available when the file is open in source mode, not in visual mode.

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
