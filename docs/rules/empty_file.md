# empty_file
::: {.callout-note title="Added in 0.6.0" .low-opacity}
:::

## What it does

Reports R files that contain no code: either truly empty, only whitespace,
or only plain comments.

Files that contain at least one roxygen comment (a line starting with `#'`)
are intentionally allowed, since packages commonly use comment-only files
as documentation templates (e.g. files in `man-roxygen/`).

## Why is this bad?

An empty or comment-only file is almost always a mistake: a placeholder that
was forgotten, or a leftover from a refactor. It adds noise to the package and
can confuse readers.

## Example

```r
# TODO: implement the data loader
```

Instead, delete the file or add the intended code.
